//! The system locale from CoreFoundation preferences.

use super::*;

// --- Locale --------------------------------------------------------------

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFPreferencesAnyApplication: CFStringRef;
    static kCFPreferencesCurrentUser: CFStringRef;
    static kCFPreferencesAnyHost: CFStringRef;
    fn CFPreferencesCopyValue(
        key: CFStringRef,
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    ) -> CFTypeRef;
}

/// The system's `AppleLocale`, read through preferences rather than by running
/// `defaults`. `None` when the domain has no such key.
pub(super) fn system_locale() -> Option<String> {
    // SAFETY: the key is a valid NUL-terminated string used only for this call.
    let key = unsafe { CFStringCreateWithCString(ptr::null(), c"AppleLocale".as_ptr(), CF_UTF8) };
    if key.is_null() {
        return None;
    }
    // SAFETY: the domain constants are exported CFStrings, and the returned
    // value is an owned copy released below.
    let value = unsafe {
        CFPreferencesCopyValue(
            key,
            kCFPreferencesAnyApplication,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        )
    };
    // SAFETY: `key` is an owned copy created above.
    unsafe { CFRelease(key) };
    if value.is_null() {
        return None;
    }
    if !is_type(value, string_type()) {
        // SAFETY: the value is an owned copy and is released once.
        unsafe { CFRelease(value) };
        return None;
    }
    // The preference value is an owned copy; read it, then release it.
    let text = cf_string(value);
    // SAFETY: the value is an owned copy and is released once.
    unsafe { CFRelease(value) };
    text
}

pub(super) fn locale() -> Detection<LocaleInfo> {
    let process = crate::detect::locale::process_from_env();
    let system = system_locale();
    crate::detect::locale::build(process, system, "CFPreferences AppleLocale")
}
