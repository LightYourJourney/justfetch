//! Graphics adapters from the IOKit registry.

use super::*;

/// An IORegistry object, retained by the call that returned it.
type IoObject = u32;

/// The IORegistry main port is always zero; the constant exists for the name.
const IO_MAIN_PORT: libc::mach_port_t = 0;
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const libc::c_char) -> CFDictionaryRef;
    fn IOServiceGetMatchingServices(
        port: libc::mach_port_t,
        matching: CFDictionaryRef,
        existing: *mut IoObject,
    ) -> libc::kern_return_t;
    fn IOIteratorNext(iterator: IoObject) -> IoObject;
    fn IOObjectRelease(object: IoObject) -> libc::kern_return_t;
    fn IORegistryEntryCreateCFProperty(
        entry: IoObject,
        key: CFStringRef,
        allocator: CFTypeRef,
        options: u32,
    ) -> CFTypeRef;
}

/// Read one value from an IOKit power-source description. The key is created
/// and released here; the description belongs to the caller's blob.
pub(super) fn registry_string(entry: IoObject, key: &CStr) -> Option<String> {
    // SAFETY: the key is a valid NUL-terminated string used only for this call.
    let key_ref = unsafe { CFStringCreateWithCString(ptr::null(), key.as_ptr(), CF_UTF8) };
    if key_ref.is_null() {
        return None;
    }
    // SAFETY: the entry is a live object from the iterator and the key is an
    // owned CFString; the property returned is an owned copy we release below.
    let value = unsafe { IORegistryEntryCreateCFProperty(entry, key_ref, ptr::null(), 0) };
    // SAFETY: `key_ref` is an owned copy created above.
    unsafe { CFRelease(key_ref) };
    if value.is_null() {
        return None;
    }
    if !is_type(value, string_type()) {
        // SAFETY: the reference is an owned copy and is released once.
        unsafe { CFRelease(value) };
        return None;
    }
    // The property is an owned copy; read it, then release it.
    let text = cf_string(value);
    // SAFETY: the property reference is an owned copy and is released once.
    unsafe { CFRelease(value) };
    text
}

/// Every IOAccelerator, which is how macOS exposes the graphics adapters.
pub(super) fn gpu() -> Detection<GpuInfo> {
    let source = "IOKit IOAccelerator";
    // SAFETY: IOServiceMatching returns a dictionary the caller owns, and
    // IOServiceGetMatchingServices consumes that reference.
    let matching = unsafe { IOServiceMatching(c"IOAccelerator".as_ptr()) };
    if matching.is_null() {
        return Detection::error(source, ErrorCode::ApiFailed, "IOServiceMatching failed");
    }
    let mut iterator: IoObject = 0;
    // SAFETY: the matching dictionary is consumed by this call, and the out
    // pointer is valid for the iterator it writes.
    let result = unsafe { IOServiceGetMatchingServices(IO_MAIN_PORT, matching, &raw mut iterator) };
    if result != 0 {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            format!("IOServiceGetMatchingServices returned {result}"),
        );
    }
    // Success with a null iterator means no service matched, and a null object
    // must not be released or iterated.
    if iterator == 0 {
        return Detection::unsupported("No graphics accelerator is registered on this machine");
    }
    let mut devices = Vec::new();
    loop {
        // SAFETY: the iterator is live; the call returns the next object or 0.
        let entry = unsafe { IOIteratorNext(iterator) };
        if entry == 0 {
            break;
        }
        let model = registry_string(entry, c"model");
        let class = registry_string(entry, c"IOClass").unwrap_or_default();
        // SAFETY: the entry is live and was returned by the iterator; releasing
        // it once is the documented ownership rule.
        unsafe { IOObjectRelease(entry) };
        // The accelerator class names the vendor and whether the adapter is on
        // the package: AGX is Apple Silicon, IntelAccelerator is an Intel
        // integrated GPU, AMDAccelerator is a discrete card on Intel Macs.
        // Anything else keeps Unknown rather than guessing.
        let (vendor, kind) = if class.starts_with("AGX") {
            (Some("Apple".to_owned()), GpuKind::Integrated)
        } else if class.starts_with("Intel") {
            (Some("Intel".to_owned()), GpuKind::Integrated)
        } else if class.starts_with("AMD") {
            (Some("AMD".to_owned()), GpuKind::Discrete)
        } else {
            (None, GpuKind::Unknown)
        };
        devices.push(GpuDevice {
            name: model,
            vendor,
            // The accelerator class is the driver's class, not a hardware id.
            id: None,
            driver: (!class.is_empty()).then(|| class.clone()),
            kind,
        });
    }
    // SAFETY: the iterator is an owned object and is released exactly once.
    unsafe { IOObjectRelease(iterator) };
    if devices.is_empty() {
        return Detection::unsupported("No graphics accelerator is registered on this machine");
    }
    Detection::available(GpuInfo { devices }, source)
}
