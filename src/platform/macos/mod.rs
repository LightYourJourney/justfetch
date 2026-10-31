use super::{PlatformData, disk, local_ip, uname, uname_identity};
use crate::model::*;
use crate::module::{ModuleId, if_selected};
use std::io;

mod cf;
mod display;
mod gpu;
mod locale;
mod packages;
mod power;
mod sysinfo;

use display::display;
use gpu::gpu;
use locale::locale;
use packages::packages;
use power::battery;
use sysinfo::{memory, swap, sysctl_string, uptime};

pub fn parent_executable() -> io::Result<String> {
    let mut buffer = [0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: getppid is a read-only operation with no pointer arguments.
    let pid = unsafe { libc::getppid() };
    // SAFETY: proc_pidpath receives the actual size of the writable byte buffer.
    let size = unsafe { libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if size <= 0 {
        return Err(io::Error::last_os_error());
    }
    let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
    String::from_utf8(buffer[..end].to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn collect(selection: &[ModuleId]) -> PlatformData {
    let data = uname();
    // The hostname is needed for the title even when no module asks for it.
    let (hostname, kernel) = uname_identity(&data);
    let os = if_selected(selection, ModuleId::Os, || {
        sysctl_string(c"kern.osproductversion").map(|version| OsInfo {
            family: OsFamily::Macos,
            name: "macOS".to_owned(),
            version: Some(version),
            id: Some("macos".to_owned()),
        })
    });
    let host = if_selected(selection, ModuleId::Host, || {
        sysctl_string(c"hw.model").map(|name| HostInfo {
            name,
            kind: HostKind::DeviceModel,
        })
    });
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel: if_selected(selection, ModuleId::Kernel, || kernel),
        uptime: if_selected(selection, ModuleId::Uptime, uptime),
        memory: if_selected(selection, ModuleId::Memory, memory),
        swap: if_selected(selection, ModuleId::Swap, swap),
        disk: if_selected(selection, ModuleId::Disk, disk),
        gpu: if_selected(selection, ModuleId::Gpu, gpu),
        display: if_selected(selection, ModuleId::Display, display),
        packages: if_selected(selection, ModuleId::Packages, packages),
        // macOS has no desktop environment in the Linux sense: the window
        // server is part of the system rather than a session a user picks.
        locale: if_selected(selection, ModuleId::Locale, locale),
        desktop: if_selected(selection, ModuleId::Desktop, || {
            Detection::unsupported(
                "macOS has no desktop environment in this sense; Aqua is the window server",
            )
        }),
        battery: if_selected(selection, ModuleId::Battery, battery),
        local_ip: if_selected(selection, ModuleId::LocalIp, local_ip),
    }
}

#[cfg(test)]
mod tests {
    use super::cf::{CF_SINT32, CFRelease, CFTypeRef, is_type, number_type, string_type};
    use std::ptr;

    unsafe extern "C" {
        // Only the type-guard test builds a CFNumber; production code only reads.
        fn CFNumberCreate(
            allocator: CFTypeRef,
            kind: libc::c_long,
            value: *const std::ffi::c_void,
        ) -> CFTypeRef;
    }

    /// `AppleLocale` is user-writable, so a preferences domain can hold a
    /// number. Sending a string message to it is an Objective-C exception that
    /// aborts the process, which is why every read checks the type first; this
    /// pins the check without touching the user's preferences.
    #[test]
    fn a_number_is_not_mistaken_for_a_string() {
        let mut value: i32 = 42;
        // SAFETY: the value pointer is valid for one i32, which is what the
        // CF_SINT32 kind declares, and the result is an owned reference.
        let number = unsafe { CFNumberCreate(ptr::null(), CF_SINT32, (&raw mut value).cast()) };
        assert!(!number.is_null());
        // SAFETY: the reference is live; both queries take it by value.
        unsafe {
            assert!(is_type(number, number_type()));
            assert!(!is_type(number, string_type()));
            CFRelease(number);
        }
    }
}
