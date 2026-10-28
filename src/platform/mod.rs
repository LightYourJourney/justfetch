#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::collect;
#[cfg(target_os = "macos")]
pub use macos::collect;
#[cfg(target_os = "windows")]
pub use windows::collect;

#[cfg(target_os = "linux")]
pub use linux::parent_executable;
#[cfg(target_os = "macos")]
pub use macos::parent_executable;
#[cfg(target_os = "windows")]
pub use windows::{logical_cpu_count, parent_executable};

#[cfg(not(windows))]
pub fn logical_cpu_count(detected: usize) -> std::io::Result<usize> {
    Ok(detected)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("justfetch v0.1 supports Linux, macOS and Windows");

use crate::model::*;

pub struct PlatformData {
    pub hostname: Detection<String>,
    pub environment: Environment,
    pub os: Detection<OsInfo>,
    pub host: Detection<HostInfo>,
    pub kernel: Detection<String>,
    pub uptime: Detection<UptimeInfo>,
    pub memory: Detection<MemoryInfo>,
    pub swap: Detection<SwapInfo>,
    pub disk: Detection<DiskInfo>,
    pub gpu: Detection<GpuInfo>,
    pub display: Detection<DisplayInfo>,
    pub battery: Detection<BatteryInfo>,
    pub local_ip: Detection<LocalIpInfo>,
}

#[cfg(unix)]
fn uname() -> Result<libc::utsname, std::io::Error> {
    let mut data = std::mem::MaybeUninit::<libc::utsname>::uninit();
    // SAFETY: uname initializes the entire properly aligned utsname on success.
    if unsafe { libc::uname(data.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: the successful call above initialized data.
    Ok(unsafe { data.assume_init() })
}

#[cfg(unix)]
fn c_chars(chars: &[libc::c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| c.to_ne_bytes()[0])
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Hostname and kernel identity from one uname result, shared by the Unix
/// adapters so both report the same fields from the same call.
#[cfg(unix)]
pub fn uname_identity(
    data: &Result<libc::utsname, std::io::Error>,
) -> (Detection<String>, Detection<String>) {
    match data {
        Ok(data) => (
            Detection::available(c_chars(&data.nodename), "uname nodename"),
            Detection::available(
                format!("{} {}", c_chars(&data.sysname), c_chars(&data.release)),
                "uname",
            ),
        ),
        Err(error) => {
            let message = error.to_string();
            let failed = || Detection::error("uname", ErrorCode::ApiFailed, &message);
            (failed(), failed())
        }
    }
}

/// The filesystem holding the user's home directory, or the root filesystem
/// when HOME is absent. Shared by the Unix adapters: `statvfs` is the same call
/// on both, and picking a different mount point per platform would invent a
/// difference the platforms do not have.
/// `fsblkcnt_t` is 32-bit on macOS and 64-bit on Linux, so the same expression
/// needs a widening conversion on one platform and an identity on the other.
#[cfg(unix)]
fn widen_blocks<T: Into<u64>>(value: T) -> u64 {
    value.into()
}

/// Loopback, link-local and unspecified addresses are assigned whether or not
/// anything is reachable, so none of them says anything about the machine.
/// Shared by the Unix and Windows adapters so both filter the same set.
fn reportable(address: std::net::Ipv4Addr) -> bool {
    !address.is_loopback() && !address.is_link_local() && !address.is_unspecified()
}

/// The path label is the only part of a disk reading that can be lossy; the
/// counters are exact. Shared so both adapters report that the same way.
fn disk_detection(info: DiskInfo, source: &str, exact_path: bool) -> Detection<DiskInfo> {
    if exact_path {
        Detection::available(info, source)
    } else {
        Detection::available_with_note(info, source, "Path is not representable as UTF-8")
    }
}

/// Convert `statvfs` counters into bytes. Pure, so the arithmetic — including
/// the cases that must be errors rather than plausible zeroes — can be tested
/// on any host.
#[cfg(unix)]
fn disk_counters(
    blocks: libc::fsblkcnt_t,
    free_blocks: libc::fsblkcnt_t,
    available_blocks: libc::fsblkcnt_t,
    fragment_size: libc::c_ulong,
    block_size: libc::c_ulong,
) -> Result<(u64, u64, u64), &'static str> {
    // f_frsize is the allocation unit and is zero on some filesystems, where
    // f_bsize is the documented substitute.
    let unit = if fragment_size > 0 {
        fragment_size
    } else {
        block_size
    };
    if unit == 0 {
        return Err("Filesystem reports a zero block size");
    }
    let total = widen_blocks(blocks)
        .checked_mul(unit)
        .ok_or("Filesystem total overflows bytes")?;
    let free = widen_blocks(free_blocks)
        .checked_mul(unit)
        .ok_or("Filesystem free overflows bytes")?;
    let available = widen_blocks(available_blocks)
        .checked_mul(unit)
        .ok_or("Filesystem available overflows bytes")?;
    let used = total
        .checked_sub(free)
        .ok_or("Filesystem free exceeds total")?;
    // A filesystem with no blocks would render as 0.00 / 0.00 GiB and read as
    // a real measurement; the Windows adapter already rejects this.
    if total == 0 {
        return Err("Filesystem reports zero total bytes");
    }
    Ok((total, used, available))
}

#[cfg(unix)]
pub fn disk() -> Detection<DiskInfo> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/".into());
    // A path that is not UTF-8 cannot be represented in JSON. The counters are
    // still exact, so the field reports them with a replacement-character path
    // and marks itself estimated rather than passing the label off as the real
    // one.
    let (reported, exact_path) = match path.to_str() {
        Some(text) => (text.to_owned(), true),
        None => (path.to_string_lossy().into_owned(), false),
    };
    let Ok(path_c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return Detection::error(
            "statvfs",
            ErrorCode::InvalidData,
            "Home path contains a NUL byte",
        );
    };
    let mut raw = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: statvfs initializes the structure it is given on success.
    if unsafe { libc::statvfs(path_c.as_ptr(), raw.as_mut_ptr()) } != 0 {
        return Detection::error(
            "statvfs",
            ErrorCode::ApiFailed,
            std::io::Error::last_os_error(),
        );
    }
    // SAFETY: the successful call above initialized the structure.
    let stats = unsafe { raw.assume_init() };
    let (total_bytes, used_bytes, available_bytes) = match disk_counters(
        stats.f_blocks,
        stats.f_bfree,
        stats.f_bavail,
        stats.f_frsize,
        stats.f_bsize,
    ) {
        Ok(counters) => counters,
        Err(message) => return Detection::error("statvfs", ErrorCode::InvalidData, message),
    };
    let info = DiskInfo {
        path: reported,
        total_bytes,
        used_bytes,
        available_bytes,
    };
    disk_detection(info, "statvfs", exact_path)
}

/// Every routable IPv4 address on an interface that is up, in the order the
/// kernel reports them. Shared by the Unix adapters.
#[cfg(unix)]
pub fn local_ip() -> Detection<LocalIpInfo> {
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs allocates a list that freeifaddrs releases below.
    if unsafe { libc::getifaddrs(&mut list) } != 0 {
        return Detection::error(
            "getifaddrs",
            ErrorCode::ApiFailed,
            std::io::Error::last_os_error(),
        );
    }
    let mut addresses = Vec::new();
    let mut cursor = list;
    while !cursor.is_null() {
        // SAFETY: getifaddrs returned a chain of valid entries ending in null.
        let item = unsafe { &*cursor };
        if let Some(address) = ipv4_of(item) {
            // SAFETY: ifa_name is a NUL-terminated string owned by the list.
            let interface = unsafe { std::ffi::CStr::from_ptr(item.ifa_name) }
                .to_string_lossy()
                .into_owned();
            addresses.push(InterfaceAddress {
                interface: (!interface.is_empty()).then_some(interface),
                address,
            });
        }
        cursor = item.ifa_next;
    }
    // SAFETY: releases the list that getifaddrs allocated.
    unsafe { libc::freeifaddrs(list) };
    if addresses.is_empty() {
        Detection::unavailable("No routable IPv4 interface")
    } else {
        Detection::available(LocalIpInfo { addresses }, "getifaddrs")
    }
}

#[cfg(unix)]
fn ipv4_of(item: &libc::ifaddrs) -> Option<std::net::Ipv4Addr> {
    if item.ifa_addr.is_null()
        || item.ifa_flags & libc::IFF_UP as u32 == 0
        || item.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
    {
        return None;
    }
    // SAFETY: ifa_addr is non-null and sa_family selects which sockaddr it is.
    if unsafe { (*item.ifa_addr).sa_family } as i32 != libc::AF_INET {
        return None;
    }
    // SAFETY: the family check above established a sockaddr_in.
    let raw = unsafe { (*item.ifa_addr.cast::<libc::sockaddr_in>()).sin_addr.s_addr };
    // s_addr holds the address in network byte order; from_be reads it back as
    // the integer Ipv4Addr expects, on either endianness.
    let address = std::net::Ipv4Addr::from(u32::from_be(raw));
    reportable(address).then_some(address)
}

#[cfg(all(unix, test))]
mod tests {
    use super::*;

    fn utsname(sysname: &str, release: &str, nodename: &str) -> libc::utsname {
        // SAFETY: utsname is a plain array-of-integer C record with a valid all
        // zero state; every field below is overwritten with NUL-terminated bytes.
        let mut data: libc::utsname = unsafe { std::mem::zeroed() };
        for (field, value) in [
            (&mut data.sysname, sysname),
            (&mut data.release, release),
            (&mut data.nodename, nodename),
        ] {
            for (slot, byte) in field.iter_mut().zip(value.bytes()) {
                *slot = byte as libc::c_char;
            }
        }
        data
    }

    #[test]
    fn disk_counters_convert_and_reject_impossible_filesystems() {
        // 1000 blocks of 4096, 400 free, 300 available to the user.
        assert_eq!(
            disk_counters(1000, 400, 300, 4096, 4096),
            Ok((4_096_000, 2_457_600, 1_228_800))
        );
        // f_frsize is zero on some filesystems; f_bsize is the substitute.
        assert_eq!(disk_counters(10, 5, 5, 0, 1024), Ok((10_240, 5_120, 5_120)));
        // These must be errors, not plausible zeroes.
        assert!(disk_counters(0, 0, 0, 4096, 4096).is_err());
        assert!(disk_counters(100, 200, 50, 4096, 4096).is_err());
        assert!(disk_counters(100, 50, 50, 0, 0).is_err());
    }

    #[test]
    fn a_lossy_path_keeps_the_counters_confirmed() {
        let info = DiskInfo {
            path: "/replaced".into(),
            total_bytes: 200,
            used_bytes: 100,
            available_bytes: 90,
        };
        match disk_detection(info.clone(), "fixture", true) {
            Detection::Available { note, .. } => assert_eq!(note, None),
            other => panic!("unexpected {other:?}"),
        }
        // The label is lossy; the counters are not. Saying "estimated" here
        // would tell a consumer the numbers are approximate.
        match disk_detection(info, "fixture", false) {
            Detection::Available {
                confidence, note, ..
            } => {
                assert_eq!(confidence, Confidence::Confirmed);
                assert!(note.is_some());
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn identity_keeps_hostname_and_kernel_separate() {
        let (hostname, kernel) = uname_identity(&Ok(utsname("Linux", "6.8.0", "example")));
        assert_eq!(hostname.value().map(String::as_str), Some("example"));
        assert_eq!(kernel.value().map(String::as_str), Some("Linux 6.8.0"));
    }

    #[test]
    fn identity_reports_one_uname_failure_in_both_fields() {
        let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let (hostname, kernel) = uname_identity(&Err(error));
        for field in [&hostname, &kernel] {
            assert!(matches!(
                field,
                Detection::Error {
                    source,
                    code: ErrorCode::ApiFailed,
                    ..
                } if source == "uname"
            ));
        }
    }
}

/// Read only: never enables raw mode, wrap settings or console flags.
#[cfg(unix)]
pub fn terminal_columns() -> Option<usize> {
    let mut size = std::mem::MaybeUninit::<libc::winsize>::uninit();
    // SAFETY: ioctl writes winsize to a correctly aligned writable object.
    if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: successful TIOCGWINSZ initializes winsize.
    let columns = unsafe { size.assume_init() }.ws_col;
    (columns > 0).then_some(usize::from(columns))
}

#[cfg(windows)]
pub fn terminal_columns() -> Option<usize> {
    use windows_sys::Win32::System::Console::*;
    let mut info = std::mem::MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::uninit();
    // SAFETY: stdout handle is borrowed; info points to writable console metadata.
    if unsafe { GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), info.as_mut_ptr()) }
        == 0
    {
        return None;
    }
    // SAFETY: successful call initialized the structure.
    let window = unsafe { info.assume_init() }.srWindow;
    usize::try_from(i32::from(window.Right) - i32::from(window.Left) + 1)
        .ok()
        .filter(|v| *v > 0)
}

#[cfg(unix)]
pub fn supports_ansi() -> bool {
    true
}

#[cfg(windows)]
pub fn supports_ansi() -> bool {
    use windows_sys::Win32::System::Console::*;
    let mut mode = 0;
    // SAFETY: borrowed stdout handle and valid mode output; does not change mode.
    unsafe {
        GetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), &mut mode) != 0
            && mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0
    }
}
