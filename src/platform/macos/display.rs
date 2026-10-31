//! Active displays from CoreGraphics.

use super::cf::*;
use crate::model::*;

// --- Displays ------------------------------------------------------------

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsMain(display: u32) -> i32;
    fn CGDisplayIsBuiltin(display: u32) -> i32;
    fn CGDisplayCopyDisplayMode(display: u32) -> CFTypeRef;
    fn CGDisplayModeGetWidth(mode: CFTypeRef) -> usize;
    fn CGDisplayModeGetHeight(mode: CFTypeRef) -> usize;
    fn CGDisplayModeGetRefreshRate(mode: CFTypeRef) -> f64;
    fn CGDisplayModeRelease(mode: CFTypeRef);
}

/// Active displays. CoreGraphics reports the pixel dimensions of the current
/// mode and whether the panel is the machine's own; it does not name a display,
/// and a virtual or remote display is reported as `unknown` rather than as an
/// external panel the API cannot prove it is.
pub(super) fn display() -> Detection<DisplayInfo> {
    let source = "CoreGraphics active displays";
    // 32 is well above the display count of any machine this runs on. If a
    // machine ever exceeded it, the extra displays would be omitted silently.
    let mut ids = [0_u32; 32];
    let mut count: u32 = 0;
    // SAFETY: the buffer holds 32 u32 values, which is the capacity passed, and
    // the count pointer is valid for one write.
    let result = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if result != 0 {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            format!("CGGetActiveDisplayList returned {result}"),
        );
    }
    let count = (count as usize).min(ids.len());
    let mut displays = Vec::new();
    for id in &ids[..count] {
        // The mode's logical size is what the desktop is laid out in and what
        // macOS itself shows as "Looks like". Neither this nor the backing
        // pixel size is the panel's native resolution (2560x1664 here), which
        // CoreGraphics does not expose without reading the EDID.
        // SAFETY: the id came from the display list the call above filled in and
        // the returned mode is an owned reference released below.
        let mode = unsafe { CGDisplayCopyDisplayMode(*id) };
        if mode.is_null() {
            continue;
        }
        // SAFETY: the mode is a live display-mode reference.
        let (width, height) = unsafe {
            (
                CGDisplayModeGetWidth(mode) as u32,
                CGDisplayModeGetHeight(mode) as u32,
            )
        };
        // A refresh rate of 0 is what this API reports on Apple Silicon, which
        // is "not known", not "0 Hz".
        // SAFETY: the mode is live; the call has no other preconditions.
        let refresh = unsafe { CGDisplayModeGetRefreshRate(mode) };
        // SAFETY: the mode is an owned copy and is released exactly once.
        unsafe { CGDisplayModeRelease(mode) };
        if width == 0 || height == 0 {
            continue;
        }
        // SAFETY: the id is from the same list; these queries take it by value.
        let (main, builtin) = unsafe { (CGDisplayIsMain(*id) != 0, CGDisplayIsBuiltin(*id) != 0) };
        displays.push(DisplayUnit {
            name: None,
            width,
            height,
            refresh_hz: (refresh > 0.0).then(|| refresh.round() as u32),
            primary: Some(main),
            kind: if builtin {
                DisplayKind::BuiltIn
            } else {
                DisplayKind::Unknown
            },
        });
    }
    if displays.is_empty() {
        // Zero active displays can mean a headless session or a sandbox that
        // denies the window server; the two look identical from here, so this
        // is "could not be answered", not "there is no display".
        return Detection::unavailable(
            "CoreGraphics reports no active display; a headless session and a sandbox that denies the window server look the same from here",
        );
    }
    Detection::available(
        DisplayInfo {
            displays,
            definition: "logical points of the current display mode".to_owned(),
        },
        source,
    )
}
