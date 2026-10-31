use super::{PlatformData, disk, local_ip, uname, uname_identity};
use crate::model::*;
use crate::module::{ModuleId, if_selected};
use std::{
    ffi::CStr,
    io, mem, ptr,
    time::{SystemTime, UNIX_EPOCH},
};

unsafe extern "C" {
    // libc exposes the other Mach primitives but not this ownership operation.
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
    // Nor this one. libc does export vm_page_size, but it holds the page size
    // this process was built for: under Rosetta that is the x86 4096, while the
    // counters host_statistics64 returns are in the host's 16384-byte pages.
    // Asking the host is the only value that matches the counters it gave us.
    fn host_page_size(host: libc::host_t, out: *mut libc::vm_size_t) -> libc::kern_return_t;
}

type CFTypeRef = *const std::ffi::c_void;
type CFArrayRef = *const std::ffi::c_void;
type CFDictionaryRef = *const std::ffi::c_void;
type CFStringRef = *const std::ffi::c_void;

/// kCFStringEncodingUTF8
const CF_UTF8: u32 = 0x0800_0100;
/// kCFNumberSInt32Type
const CF_SINT32: libc::c_long = 3;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: CFArrayRef) -> libc::c_long;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: libc::c_long) -> CFTypeRef;
    fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
    fn CFStringCreateWithCString(
        allocator: CFTypeRef,
        text: *const libc::c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFStringGetLength(string: CFStringRef) -> libc::c_long;
    fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut libc::c_char,
        size: libc::c_long,
        encoding: u32,
    ) -> u8;
    // CFNumberType is CF_ENUM(CFIndex), which is a long, not an int.
    fn CFNumberGetValue(number: CFTypeRef, kind: libc::c_long, out: *mut std::ffi::c_void) -> u8;
    fn CFBooleanGetValue(value: CFTypeRef) -> u8;
    fn CFGetTypeID(value: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFRelease(value: CFTypeRef);
}

fn cf_string(value: CFStringRef) -> Option<String> {
    // SAFETY: the value was checked to be a CFString by the caller.
    let length = unsafe { CFStringGetLength(value) };
    if length < 0 {
        return None;
    }
    // UTF-8 needs at most 3 bytes per UTF-16 unit, plus the terminator.
    let capacity = (length as usize).saturating_mul(3).saturating_add(1);
    let mut buffer = vec![0_i8; capacity];
    // SAFETY: the buffer size matches the capacity passed and the value was
    // checked to be a CFString. A non-zero return means the text fitted.
    let ok = unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr(),
            buffer.len() as libc::c_long,
            CF_UTF8,
        )
    };
    if ok == 0 {
        return None;
    }
    // SAFETY: the call above wrote a NUL-terminated string into the buffer.
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// Whether a Core Foundation value is of the expected type. Sending a string
/// message to a number is an Objective-C exception, not an error code, and it
/// aborts the process — so the type is checked before the call, not after.
fn is_type(value: CFTypeRef, expected: usize) -> bool {
    // SAFETY: every CF value has a type id; the call has no preconditions.
    unsafe { CFGetTypeID(value) == expected }
}

/// Type ids are constant for the life of the process and the queries take no
/// arguments, so they are wrapped rather than repeated as bare unsafe blocks.
fn string_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFStringGetTypeID() }
}

fn number_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFNumberGetTypeID() }
}

fn boolean_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFBooleanGetTypeID() }
}

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
unsafe extern "C" {
    // Only the type-guard test builds a CFNumber; production code only reads.
    fn CFNumberCreate(
        allocator: CFTypeRef,
        kind: libc::c_long,
        value: *const std::ffi::c_void,
    ) -> CFTypeRef;
}

#[cfg(test)]
mod tests {
    use super::*;

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
