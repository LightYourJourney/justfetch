use super::{PlatformData, disk_detection, reportable};
use crate::model::*;
use std::io;
use sysinfo::System;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub fn logical_cpu_count(_detected: usize) -> io::Result<usize> {
    use windows_sys::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};
    // SAFETY: pure system query with the documented all-groups selector.
    let count = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if count == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(count as usize)
    }
}

pub fn parent_executable() -> io::Result<String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::{Diagnostics::ToolHelp::*, Threading::*},
    };
    struct OwnedHandle(HANDLE);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns exactly one validated native handle.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    // One documented ToolHelp snapshot obtains the parent ID. No per-process
    // metadata scan or sysinfo refresh; stop as soon as our own entry is found.
    // SAFETY: read-only snapshot of process IDs; no borrowed pointer arguments.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = OwnedHandle(snapshot);
    // SAFETY: PROCESSENTRY32W is an integer/array C record with valid zero state.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    // SAFETY: validated snapshot handle and correctly sized writable entry.
    if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let own_pid = std::process::id();
    while entry.th32ProcessID != own_pid {
        // SAFETY: entry remains initialized with its size, snapshot is live.
        if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: request only query rights; ID came from a native process snapshot.
    let parent = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            entry.th32ParentProcessID,
        )
    };
    if parent.is_null() {
        return Err(io::Error::last_os_error());
    }
    let parent = OwnedHandle(parent);
    let mut buffer = vec![0_u16; 32768];
    let mut size = buffer.len() as u32;
    // SAFETY: live handle, writable UTF-16 buffer, exact capacity in code units.
    if unsafe { QueryFullProcessImageNameW(parent.0, 0, buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    String::from_utf16(&buffer[..size as usize])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn memory() -> Detection<MemoryInfo> {
    // SAFETY: MEMORYSTATUSEX is an integer-only C struct with valid all-zero state.
    let mut data: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    data.dwLength = std::mem::size_of_val(&data) as u32;
    // SAFETY: data is aligned, writable, and dwLength specifies its exact size.
    if unsafe { GlobalMemoryStatusEx(&mut data) } == 0 {
        return Detection::error(
            "GlobalMemoryStatusEx",
            ErrorCode::ApiFailed,
            std::io::Error::last_os_error(),
        );
    }
    match data.ullTotalPhys.checked_sub(data.ullAvailPhys) {
        Some(used_bytes) if data.ullTotalPhys > 0 => Detection::available(
            MemoryInfo {
                total_bytes: data.ullTotalPhys,
                used_bytes,
                definition: MemoryDefinition::TotalMinusAvailable,
                scope: Scope::OperatingSystem,
            },
            "GlobalMemoryStatusEx",
        ),
        _ => Detection::error(
            "GlobalMemoryStatusEx",
            ErrorCode::InvalidData,
            "Inconsistent physical memory counters",
        ),
    }
}

fn disk() -> Detection<DiskInfo> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let path = std::env::var_os("USERPROFILE")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("SystemDrive"))
        .unwrap_or_else(|| "C:\\".into());
    // A path that is not representable in UTF-8 cannot go into JSON verbatim;
    // the counters are still exact, so the field reports them with a
    // replacement-character path and marks itself estimated.
    let (reported, exact_path) = match path.to_str() {
        Some(text) => (text.to_owned(), true),
        None => (path.to_string_lossy().into_owned(), false),
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut available = 0_u64;
    let mut total = 0_u64;
    let mut free = 0_u64;
    // SAFETY: the path is NUL-terminated and each output is a writable u64.
    let result =
        unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    if result == 0 {
        return Detection::error(
            "GetDiskFreeSpaceExW",
            ErrorCode::ApiFailed,
            io::Error::last_os_error(),
        );
    }
    let Some(used_bytes) = total
        .checked_sub(free)
        .filter(|used| total > 0 && *used <= total)
    else {
        return Detection::error(
            "GetDiskFreeSpaceExW",
            ErrorCode::InvalidData,
            "Inconsistent filesystem counters",
        );
    };
    let info = DiskInfo {
        path: reported,
        total_bytes: total,
        used_bytes,
        available_bytes: available,
    };
    disk_detection(info, "GetDiskFreeSpaceExW", exact_path)
}

/// Windows exposes a commit limit rather than a page-file usage figure, and the
/// obvious derivation from it is unsound: committed memory can be *smaller* than
/// resident memory, because file-backed mappings and driver memory are resident
/// without being committed. Measured on a CI runner the difference went
/// negative — 2.41 GiB committed against 2.62 GiB resident — so the subtraction
/// has no meaning to report. No API this project permits exposes page-file usage
/// directly (WMI would be an out-of-process query, PDH a separate subsystem), so
/// the field says it is unsupported instead of inventing a number.
fn swap() -> Detection<SwapInfo> {
    Detection::unsupported(
        "Windows reports a commit limit, not page-file usage, and no permitted API exposes the latter",
    )
}

fn local_ip() -> Detection<LocalIpInfo> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    // The call fills this buffer with IP_ADAPTER_ADDRESSES_LH records, so the
    // allocation must be at least as aligned as that struct; a Vec<u8> promises
    // only byte alignment, so the buffer is allocated in u64 units instead.
    const INITIAL_BYTES: u32 = 16 * 1024;
    let mut buffer = vec![0_u64; (INITIAL_BYTES / 8) as usize];
    let mut size = INITIAL_BYTES;
    // SAFETY: the buffer is writable for `size` bytes and correctly aligned.
    let mut result = unsafe {
        GetAdaptersAddresses(
            u32::from(AF_UNSPEC),
            flags,
            std::ptr::null(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result == ERROR_BUFFER_OVERFLOW {
        // `size` now holds the byte count the failed call requires. The buffer
        // is grown to whole u64 units, so it is at least that large; passing the
        // requirement back unchanged keeps the contract the call expects.
        buffer.resize((size as usize).div_ceil(8), 0);
        // SAFETY: the buffer now holds at least `size` bytes and is aligned.
        result = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC),
                flags,
                std::ptr::null(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
    }
    if result != 0 {
        return Detection::error(
            "GetAdaptersAddresses",
            ErrorCode::ApiFailed,
            format!("Win32 error {result}"),
        );
    }
    let mut addresses = Vec::new();
    let mut adapter = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !adapter.is_null() {
        // SAFETY: the successful call above returned a chain of valid adapters.
        let entry = unsafe { &*adapter };
        if entry.OperStatus == IfOperStatusUp {
            let interface = if entry.FriendlyName.is_null() {
                None
            } else {
                // SAFETY: FriendlyName is a NUL-terminated UTF-16 string.
                let name = unsafe { wide_to_string(entry.FriendlyName) };
                (!name.is_empty()).then_some(name)
            };
            let mut unicast = entry.FirstUnicastAddress;
            while !unicast.is_null() {
                // SAFETY: the chain above is owned by the adapter list.
                let node = unsafe { &*unicast };
                if let Some(address) = ipv4_of(&node.Address) {
                    addresses.push(InterfaceAddress {
                        interface: interface.clone(),
                        address,
                    });
                }
                unicast = node.Next;
            }
        }
        adapter = entry.Next;
    }
    if addresses.is_empty() {
        Detection::unavailable("No routable IPv4 interface")
    } else {
        Detection::available(LocalIpInfo { addresses }, "GetAdaptersAddresses")
    }
}

/// # Safety
/// `pointer` must point at a NUL-terminated UTF-16 string, or the scan stops at
/// the first NUL within 1024 code units.
unsafe fn wide_to_string(pointer: *const u16) -> String {
    let mut length = 0;
    // SAFETY: the caller guarantees the string starts at `pointer`; the bound
    // stops a malformed or unterminated value from scanning without limit.
    while length < 1024 && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    // SAFETY: the loop above established `length` readable code units.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) })
}

fn ipv4_of(
    address: &windows_sys::Win32::Networking::WinSock::SOCKET_ADDRESS,
) -> Option<std::net::Ipv4Addr> {
    use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    if address.lpSockaddr.is_null() {
        return None;
    }
    // SAFETY: lpSockaddr points at a sockaddr whose family selects its layout.
    if unsafe { (*address.lpSockaddr).sa_family } != AF_INET {
        return None;
    }
    // SAFETY: the family check above established a sockaddr_in.
    let v4 = unsafe { &*address.lpSockaddr.cast::<SOCKADDR_IN>() };
    // SAFETY: S_addr is the active member of the address union on AF_INET and
    // holds the address in network byte order.
    let raw = unsafe { v4.sin_addr.S_un.S_addr };
    let parsed = std::net::Ipv4Addr::from(u32::from_be(raw));
    reportable(parsed).then_some(parsed)
}

pub fn collect() -> PlatformData {
    let hostname = match System::host_name().filter(|s| !s.is_empty()) {
        Some(name) => Detection::available(name, "sysinfo GetComputerNameExW"),
        None => Detection::unavailable("Hostname is not exposed by the Windows adapter"),
    };
    let build = System::kernel_version().filter(|s| !s.is_empty());
    let os = match (System::long_os_version(), &build) {
        (Some(name), Some(build)) => Detection::available(
            OsInfo {
                family: OsFamily::Windows,
                name,
                version: Some(format!("build {build}")),
                id: Some("windows".into()),
            },
            "sysinfo Windows version registry",
        ),
        _ => Detection::unavailable("Windows product name or build registry value unavailable"),
    };
    let kernel = match build {
        Some(build) => Detection::available(
            format!("Windows NT build {build}"),
            "sysinfo Windows CurrentBuildNumber",
        ),
        None => Detection::unavailable("Windows kernel build is unavailable"),
    };
    let host = hostname.clone().map(|name| HostInfo {
        name,
        kind: HostKind::Hostname,
    });
    let uptime = Detection::available(
        UptimeInfo {
            seconds: System::uptime(),
            scope: Scope::OperatingSystem,
        },
        "GetTickCount64",
    );
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel,
        uptime,
        memory: memory(),
        swap: swap(),
        disk: disk(),
        local_ip: local_ip(),
    }
}
