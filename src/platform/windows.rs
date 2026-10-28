use super::{PlatformData, disk_detection, reportable};
use crate::model::*;
use crate::module::{ModuleId, if_selected, wanted};
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
    // The family selects the layout, but only the declared length says the
    // buffer is actually that large; a short one would make the cast below read
    // past it.
    if address.iSockaddrLength < std::mem::size_of::<SOCKADDR_IN>() as i32 {
        return None;
    }
    // SAFETY: lpSockaddr points at a sockaddr whose family selects its layout.
    if unsafe { (*address.lpSockaddr).sa_family } != AF_INET {
        return None;
    }
    // SAFETY: the family check and the length check above established a
    // sockaddr_in of at least its full size.
    let v4 = unsafe { &*address.lpSockaddr.cast::<SOCKADDR_IN>() };
    // SAFETY: S_addr is the active member of the address union on AF_INET and
    // holds the address in network byte order.
    let raw = unsafe { v4.sin_addr.S_un.S_addr };
    let parsed = std::net::Ipv4Addr::from(u32::from_be(raw));
    reportable(parsed).then_some(parsed)
}

/// Windows reports one aggregate battery through `GetSystemPowerStatus`, not
/// per-unit detail, so this is one entry even on a machine with two packs.
fn battery() -> Detection<BatteryInfo> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = std::mem::MaybeUninit::<SYSTEM_POWER_STATUS>::uninit();
    // SAFETY: the call writes one SYSTEM_POWER_STATUS into the pointer it is
    // given, which is valid and correctly aligned for that type.
    if unsafe { GetSystemPowerStatus(status.as_mut_ptr()) } == 0 {
        return Detection::error(
            "GetSystemPowerStatus",
            ErrorCode::ApiFailed,
            io::Error::last_os_error(),
        );
    }
    // SAFETY: the call succeeded, so the struct is fully initialized.
    let status = unsafe { status.assume_init() };
    // BatteryFlag 255 means the value is unknown — neither "there is a
    // battery" nor "there is none", so it is not turned into a unit.
    if status.BatteryFlag == 255 {
        return Detection::unavailable("Windows reports an unknown battery state");
    }
    // 128 means no system battery.
    if status.BatteryFlag & 128 != 0 {
        return Detection::unsupported("Windows reports no system battery on this machine");
    }
    let percent = (status.BatteryLifePercent != 255).then_some(status.BatteryLifePercent);
    let state = if status.BatteryFlag & 8 != 0 {
        // Bit 3 is "charging"; a full pack also clears it.
        BatteryState::Charging
    } else if percent == Some(100) {
        BatteryState::Full
    } else if status.ACLineStatus == 0 {
        BatteryState::Discharging
    } else {
        BatteryState::Unknown
    };
    let power = match status.ACLineStatus {
        0 => PowerSource::Battery,
        1 => PowerSource::Ac,
        _ => PowerSource::Unknown,
    };
    Detection::available(
        BatteryInfo {
            units: vec![BatteryUnit {
                name: "System battery".to_owned(),
                percent,
                state,
            }],
            power,
        },
        "GetSystemPowerStatus",
    )
}

/// Graphics adapters as GDI reports them. `windows-sys` ships no DXGI bindings,
/// and this project does not take a second Windows binding crate for one
/// module, so the source is `EnumDisplayDevicesW`: it names the adapters
/// Windows has attached to a display. An adapter with no display attached is
/// therefore not listed, which the compatibility matrix records.
fn gpu() -> Detection<GpuInfo> {
    use windows_sys::Win32::Graphics::Gdi::{
        DISPLAY_DEVICE_ACTIVE, DISPLAY_DEVICEW, EnumDisplayDevicesW,
    };

    let wide = |buffer: &[u16]| {
        let end = buffer
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end]).trim().to_owned()
    };
    let mut devices: Vec<GpuDevice> = Vec::new();
    let mut index = 0_u32;
    loop {
        let mut device = std::mem::MaybeUninit::<DISPLAY_DEVICEW>::zeroed();
        // SAFETY: the struct is plain old data; writing `cb` through the
        // uninitialized pointer is valid for that field.
        unsafe {
            (*device.as_mut_ptr()).cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        }
        // SAFETY: the buffer is sized as `cb` declares and the call only writes
        // into it; a zero return means the index is past the last adapter.
        let found = unsafe { EnumDisplayDevicesW(std::ptr::null(), index, device.as_mut_ptr(), 0) };
        if found == 0 {
            break;
        }
        index += 1;
        // SAFETY: the call succeeded, so the struct is fully initialized.
        let device = unsafe { device.assume_init() };
        if device.StateFlags & DISPLAY_DEVICE_ACTIVE == 0 {
            continue;
        }
        let name = wide(&device.DeviceString);
        if name.is_empty() {
            continue;
        }
        let id = wide(&device.DeviceID);
        // One adapter behind several monitors appears once per monitor with the
        // same DeviceID; two identical cards have different ones. Without an
        // id the name is the only key the entry will have.
        let duplicate = if id.is_empty() {
            devices
                .iter()
                .any(|existing| existing.name.as_deref() == Some(name.as_str()))
        } else {
            devices
                .iter()
                .any(|existing| existing.id.as_deref() == Some(id.as_str()))
        };
        if duplicate {
            continue;
        }
        // The id looks like `PCI\VEN_10DE&DEV_2504&SUBSYS_…`; the vendor is the
        // four hex digits after `VEN_`, wherever they fall in the string.
        let vendor = crate::detect::hardware::pci_vendor(&id);
        devices.push(GpuDevice {
            name: Some(name),
            vendor: vendor.map(str::to_owned),
            id: (!id.is_empty()).then_some(id),
            driver: None,
            kind: GpuKind::Unknown,
        });
    }
    if devices.is_empty() {
        return Detection::unavailable("EnumDisplayDevicesW reports no active display adapter");
    }
    Detection::available(GpuInfo { devices }, "EnumDisplayDevicesW")
}

/// Active monitors from GDI. `GetMonitorInfoW` gives the desktop rectangle,
/// which is the resolution the user sees, and the primary flag; the refresh
/// rate comes from the display settings of the same device.
fn display() -> Detection<DisplayInfo> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW,
        GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _device: HDC,
        _rect: *mut RECT,
        data: isize,
    ) -> i32 {
        // SAFETY: `data` is the pointer to the Vec the caller passed, and the
        // callback is called synchronously while it is alive.
        let displays = unsafe { &mut *(data as *mut Vec<DisplayUnit>) };
        let mut info = std::mem::MaybeUninit::<MONITORINFOEXW>::zeroed();
        // SAFETY: writing cbSize through the uninitialized pointer is valid for
        // that field, and the struct is plain data.
        unsafe {
            (*info.as_mut_ptr()).monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        }
        // SAFETY: the buffer is sized as cbSize declares and the handle is one
        // GDI passes to this callback.
        if unsafe { GetMonitorInfoW(monitor, info.as_mut_ptr().cast()) } == 0 {
            return 1;
        }
        // SAFETY: the successful call initialized the struct.
        let info = unsafe { info.assume_init() };
        let rect = info.monitorInfo.rcMonitor;
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        if width == 0 || height == 0 {
            return 1;
        }
        // The refresh rate belongs to the device the monitor is on; a failure
        // leaves it unknown rather than assumed.
        let mut mode = std::mem::MaybeUninit::<DEVMODEW>::zeroed();
        // SAFETY: writing dmSize through the uninitialized pointer is valid for
        // that field.
        unsafe {
            (*mode.as_mut_ptr()).dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        }
        // SAFETY: the device name is NUL-terminated UTF-16 from the struct
        // above and the mode buffer is sized as dmSize declares.
        let refresh = unsafe {
            EnumDisplaySettingsW(
                info.szDevice.as_ptr(),
                ENUM_CURRENT_SETTINGS,
                mode.as_mut_ptr(),
            )
        };
        let refresh_hz = if refresh == 0 {
            None
        } else {
            // SAFETY: the call succeeded, so the struct is fully initialized.
            let mode = unsafe { mode.assume_init() };
            (mode.dmDisplayFrequency > 1).then_some(mode.dmDisplayFrequency)
        };
        displays.push(DisplayUnit {
            name: None,
            width,
            height,
            refresh_hz,
            primary: Some(info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0),
            kind: DisplayKind::Unknown,
        });
        1
    }

    let mut displays: Vec<DisplayUnit> = Vec::new();
    // SAFETY: the callback is a valid function with the documented signature and
    // `data` points at a Vec that outlives the synchronous enumeration.
    let found = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(collect),
            (&raw mut displays) as isize,
        )
    };
    if found == 0 || displays.is_empty() {
        return Detection::unavailable("EnumDisplayMonitors reports no active monitor");
    }
    Detection::available(
        DisplayInfo {
            displays,
            definition: "desktop pixels reported by GetMonitorInfoW".to_owned(),
        },
        "EnumDisplayMonitors",
    )
}

/// Scoop's installed applications. winget has no local database this build can
/// read: its manifest cache is not a package list, so it is reported through
/// the module's unsupported state rather than counted from something else.
fn packages() -> Detection<PackagesInfo> {
    let Some(home) = std::env::var_os("USERPROFILE") else {
        return Detection::unavailable("USERPROFILE is not set, so Scoop's directory is unknown");
    };
    let apps = std::path::PathBuf::from(home).join("scoop").join("apps");
    let Ok(entries) = std::fs::read_dir(&apps) else {
        return Detection::unsupported(
            "No Scoop installation was found, and winget has no local package database to read",
        );
    };
    let count = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .count() as u64;
    if count == 0 {
        return Detection::unsupported("Scoop's apps directory is empty");
    }
    Detection::available(
        PackagesInfo {
            managers: vec![PackageManager {
                name: "scoop".to_owned(),
                count,
                definition: "installed applications: one directory in scoop/apps".to_owned(),
            }],
        },
        "scoop/apps directories",
    )
}

/// The process locale from the POSIX variables, and the user's default locale
/// from Windows. `GetUserDefaultLocaleName` answers for the user profile, which
/// is what the next login will use; the environment answers for this process.
fn locale() -> Detection<LocaleInfo> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;

    let var = |name: &str| std::env::var(name).ok();
    let lc_all = var("LC_ALL");
    let lc_messages = var("LC_MESSAGES");
    let lang = var("LANG");
    let process = crate::detect::locale::process_locale(
        lc_all.as_deref(),
        lc_messages.as_deref(),
        lang.as_deref(),
    );
    // LOCALE_NAME_MAX_LENGTH is 85, including the terminator.
    let mut buffer = [0_u16; 85];
    // SAFETY: the buffer holds the documented maximum length and the size is
    // passed in characters.
    let written = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    let system = (written > 0).then(|| {
        let end = (written as usize - 1).min(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    });
    crate::detect::locale::build(process, system, "GetUserDefaultLocaleName")
}

pub fn collect(selection: &[ModuleId]) -> PlatformData {
    // The hostname is needed for the title even when no module asks for it.
    let hostname = match System::host_name().filter(|s| !s.is_empty()) {
        Some(name) => Detection::available(name, "sysinfo GetComputerNameExW"),
        None => Detection::unavailable("Hostname is not exposed by the Windows adapter"),
    };
    // `long_os_version` and `kernel_version` both read the registry, so they are
    // only asked for when the OS or Kernel row is actually wanted.
    let wants_build = wanted(selection, ModuleId::Os) || wanted(selection, ModuleId::Kernel);
    let build = wants_build
        .then(|| System::kernel_version().filter(|s| !s.is_empty()))
        .flatten();
    let os = if_selected(selection, ModuleId::Os, || {
        match (System::long_os_version(), &build) {
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
        }
    });
    let kernel = if_selected(selection, ModuleId::Kernel, || match build {
        Some(build) => Detection::available(
            format!("Windows NT build {build}"),
            "sysinfo Windows CurrentBuildNumber",
        ),
        None => Detection::unavailable("Windows kernel build is unavailable"),
    });
    let host = if_selected(selection, ModuleId::Host, || {
        hostname.clone().map(|name| HostInfo {
            name,
            kind: HostKind::Hostname,
        })
    });
    let uptime = if_selected(selection, ModuleId::Uptime, || {
        Detection::available(
            UptimeInfo {
                seconds: System::uptime(),
                scope: Scope::OperatingSystem,
            },
            "GetTickCount64",
        )
    });
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel,
        uptime,
        memory: if_selected(selection, ModuleId::Memory, memory),
        swap: if_selected(selection, ModuleId::Swap, swap),
        disk: if_selected(selection, ModuleId::Disk, disk),
        gpu: if_selected(selection, ModuleId::Gpu, gpu),
        display: if_selected(selection, ModuleId::Display, display),
        packages: if_selected(selection, ModuleId::Packages, packages),
        locale: if_selected(selection, ModuleId::Locale, locale),
        desktop: if_selected(selection, ModuleId::Desktop, || {
            Detection::unsupported(
                "Windows has no desktop environment in this sense; the shell is part of the system",
            )
        }),
        battery: if_selected(selection, ModuleId::Battery, battery),
        local_ip: if_selected(selection, ModuleId::LocalIp, local_ip),
    }
}
