//! Battery and mains state from IOKit's power-source tree.

use super::cf::*;
use crate::model::*;
use std::ffi::CStr;
use std::ptr;

// IOKit is the only supported source for battery state on macOS. There is no
// sysctl for it and the alternative, `pmset`, is a subprocess.

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
}

/// Read one value from an IOKit power-source description. The key is created
/// and released here; the description belongs to the caller's blob.
fn power_value(description: CFDictionaryRef, key: &CStr) -> Option<CFTypeRef> {
    // SAFETY: the key is a valid NUL-terminated string and `key` is only used
    // for the duration of the call. A null result is checked by the caller.
    let key_ref = unsafe { CFStringCreateWithCString(ptr::null(), key.as_ptr(), CF_UTF8) };
    if key_ref.is_null() {
        return None;
    }
    // SAFETY: both references come from successful calls and outlive this call.
    let value = unsafe { CFDictionaryGetValue(description, key_ref) };
    // SAFETY: `key_ref` is an owned copy created above and must be released.
    unsafe { CFRelease(key_ref) };
    (!value.is_null()).then_some(value)
}

fn power_string(description: CFDictionaryRef, key: &CStr) -> Option<String> {
    let value = power_value(description, key)?;
    if !is_type(value, string_type()) {
        return None;
    }
    // The dictionary owns the value; `cf_string` does not release it.
    cf_string(value)
}

fn power_int(description: CFDictionaryRef, key: &CStr) -> Option<i32> {
    let value = power_value(description, key)?;
    if !is_type(value, number_type()) {
        return None;
    }
    let mut out = 0_i32;
    // SAFETY: the output pointer is valid for the write the call performs, and
    // the value is a CFNumber as its key documents.
    let ok = unsafe { CFNumberGetValue(value, CF_SINT32, (&raw mut out).cast()) };
    (ok != 0).then_some(out)
}

fn power_bool(description: CFDictionaryRef, key: &CStr) -> Option<bool> {
    let value = power_value(description, key)?;
    if !is_type(value, boolean_type()) {
        return None;
    }
    // SAFETY: the value is a CFBoolean as its key documents.
    Some(unsafe { CFBooleanGetValue(value) } != 0)
}

pub(super) fn battery() -> Detection<BatteryInfo> {
    let source = "IOKit power sources";
    // SAFETY: the copy functions return owned references; both are released
    // before returning, including on the early paths below.
    let blob = unsafe { IOPSCopyPowerSourcesInfo() };
    if blob.is_null() {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            "IOPSCopyPowerSourcesInfo failed",
        );
    }
    // SAFETY: `blob` is a live reference from the call above.
    let list = unsafe { IOPSCopyPowerSourcesList(blob) };
    if list.is_null() {
        // SAFETY: `blob` is an owned copy and has not been released yet.
        unsafe { CFRelease(blob) };
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            "IOPSCopyPowerSourcesList failed",
        );
    }
    // SAFETY: `list` is a live array reference from the call above.
    let count = unsafe { CFArrayGetCount(list) };
    let mut units = Vec::new();
    let mut power = PowerSource::Unknown;
    let mut internal_sources = 0;
    for index in 0..count {
        // SAFETY: index is within the array's count; the borrowed description
        // is valid while the blob lives.
        let item = unsafe { CFArrayGetValueAtIndex(list, index) };
        if item.is_null() {
            continue;
        }
        // SAFETY: `item` came from `list`, which belongs to `blob`, so the
        // description borrows from the same live blob.
        let description = unsafe { IOPSGetPowerSourceDescription(blob, item) };
        if description.is_null() {
            continue;
        }
        // A UPS is a power source but not the machine's battery.
        if power_string(description, c"Transport Type").as_deref() != Some("Internal") {
            continue;
        }
        internal_sources += 1;
        // A two-bay portable publishes both bays; the empty one says so.
        if power_bool(description, c"Is Present") == Some(false) {
            continue;
        }
        if let Some(state) = power_string(description, c"Power Source State") {
            power = match state.as_str() {
                "AC Power" => PowerSource::Ac,
                "Battery Power" => PowerSource::Battery,
                _ => PowerSource::Unknown,
            };
        }
        let current = power_int(description, c"Current Capacity");
        let maximum = power_int(description, c"Max Capacity").filter(|max| *max > 0);
        // A charge above the maximum is not a percentage; it is not scaled into
        // one either, because that would make an invalid counter look valid.
        let percent = match (current, maximum) {
            (Some(current), Some(maximum)) if (0..=maximum).contains(&current) => {
                Some(((f64::from(current) / f64::from(maximum)) * 100.0).round() as u8)
            }
            _ => None,
        };
        let charging = power_bool(description, c"Is Charging");
        let charged = power_bool(description, c"Is Charged");
        let state = if charged == Some(true) || percent == Some(100) {
            BatteryState::Full
        } else if charging == Some(true) {
            BatteryState::Charging
        } else if charging == Some(false) && power == PowerSource::Battery {
            BatteryState::Discharging
        } else {
            // A missing flag is not evidence of discharging.
            BatteryState::Unknown
        };
        units.push(BatteryUnit {
            name: power_string(description, c"Name").unwrap_or_else(|| "Battery".to_owned()),
            percent,
            state,
        });
    }
    // SAFETY: both references were created by the copy functions above and are
    // released exactly once here.
    unsafe {
        CFRelease(list);
        CFRelease(blob);
    }
    if units.is_empty() {
        return Detection::unsupported(if internal_sources > 0 {
            "The internal battery is reported as not present"
        } else {
            "This machine reports no internal battery; macOS desktops have none"
        });
    }
    Detection::available(BatteryInfo { units, power }, source)
}
