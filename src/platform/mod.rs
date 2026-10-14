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
