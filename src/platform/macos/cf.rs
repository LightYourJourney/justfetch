//! The Core Foundation layer shared by the macOS subsystems.
//!
//! Every declaration here is used by at least two of `power`, `gpu`, `display`
//! and `locale`; anything used by one subsystem lives with it.

use std::ffi::CStr;

pub(super) type CFTypeRef = *const std::ffi::c_void;
pub(super) type CFArrayRef = *const std::ffi::c_void;
pub(super) type CFDictionaryRef = *const std::ffi::c_void;
pub(super) type CFStringRef = *const std::ffi::c_void;

/// kCFStringEncodingUTF8
pub(super) const CF_UTF8: u32 = 0x0800_0100;
/// kCFNumberSInt32Type
pub(super) const CF_SINT32: libc::c_long = 3;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub(super) fn CFArrayGetCount(array: CFArrayRef) -> libc::c_long;
    pub(super) fn CFArrayGetValueAtIndex(array: CFArrayRef, index: libc::c_long) -> CFTypeRef;
    pub(super) fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
    pub(super) fn CFStringCreateWithCString(
        allocator: CFTypeRef,
        text: *const libc::c_char,
        encoding: u32,
    ) -> CFStringRef;
    pub(super) fn CFStringGetLength(string: CFStringRef) -> libc::c_long;
    pub(super) fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut libc::c_char,
        size: libc::c_long,
        encoding: u32,
    ) -> u8;
    // CFNumberType is CF_ENUM(CFIndex), which is a long, not an int.
    pub(super) fn CFNumberGetValue(
        number: CFTypeRef,
        kind: libc::c_long,
        out: *mut std::ffi::c_void,
    ) -> u8;
    pub(super) fn CFBooleanGetValue(value: CFTypeRef) -> u8;
    pub(super) fn CFGetTypeID(value: CFTypeRef) -> usize;
    pub(super) fn CFStringGetTypeID() -> usize;
    pub(super) fn CFNumberGetTypeID() -> usize;
    pub(super) fn CFBooleanGetTypeID() -> usize;
    pub(super) fn CFRelease(value: CFTypeRef);
}

pub(super) fn cf_string(value: CFStringRef) -> Option<String> {
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
pub(super) fn is_type(value: CFTypeRef, expected: usize) -> bool {
    // SAFETY: every CF value has a type id; the call has no preconditions.
    unsafe { CFGetTypeID(value) == expected }
}

/// Type ids are constant for the life of the process and the queries take no
/// arguments, so they are wrapped rather than repeated as bare unsafe blocks.
pub(super) fn string_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFStringGetTypeID() }
}

pub(super) fn number_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFNumberGetTypeID() }
}

pub(super) fn boolean_type() -> usize {
    // SAFETY: the call returns a constant type id and has no preconditions.
    unsafe { CFBooleanGetTypeID() }
}
