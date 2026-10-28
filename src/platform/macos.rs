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

// --- Power sources -------------------------------------------------------
//
// IOKit is the only supported source for battery state on macOS. There is no
// sysctl for it and the alternative, `pmset`, is a subprocess.

type CFTypeRef = *const std::ffi::c_void;
type CFArrayRef = *const std::ffi::c_void;
type CFDictionaryRef = *const std::ffi::c_void;
type CFStringRef = *const std::ffi::c_void;

/// kCFStringEncodingUTF8
const CF_UTF8: u32 = 0x0800_0100;
/// kCFNumberSInt32Type
const CF_SINT32: i32 = 3;

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
    fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut libc::c_char,
        size: libc::c_long,
        encoding: u32,
    ) -> u8;
    fn CFNumberGetValue(number: CFTypeRef, kind: i32, out: *mut std::ffi::c_void) -> u8;
    fn CFBooleanGetValue(value: CFTypeRef) -> u8;
    fn CFRelease(value: CFTypeRef);
}

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
    let mut buffer = [0_i8; 128];
    // SAFETY: the buffer size matches the capacity passed, and the value is a
    // CFString as its key documents. A non-zero return means the text fitted.
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

fn power_int(description: CFDictionaryRef, key: &CStr) -> Option<i32> {
    let value = power_value(description, key)?;
    let mut out = 0_i32;
    // SAFETY: the output pointer is valid for the write the call performs, and
    // the value is a CFNumber as its key documents.
    let ok = unsafe { CFNumberGetValue(value, CF_SINT32, (&raw mut out).cast()) };
    (ok != 0).then_some(out)
}

fn power_bool(description: CFDictionaryRef, key: &CStr) -> Option<bool> {
    let value = power_value(description, key)?;
    // SAFETY: the value is a CFBoolean as its key documents.
    Some(unsafe { CFBooleanGetValue(value) } != 0)
}

fn battery() -> Detection<BatteryInfo> {
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
        if let Some(state) = power_string(description, c"Power Source State") {
            power = match state.as_str() {
                "AC Power" => PowerSource::Ac,
                "Battery Power" => PowerSource::Battery,
                _ => PowerSource::Unknown,
            };
        }
        let current = power_int(description, c"Current Capacity");
        let maximum = power_int(description, c"Max Capacity").filter(|max| *max > 0);
        let percent = match (current, maximum) {
            (Some(current), Some(maximum)) => Some(
                ((f64::from(current) / f64::from(maximum)) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8,
            ),
            _ => None,
        };
        let charging = power_bool(description, c"Is Charging").unwrap_or(false);
        let charged = power_bool(description, c"Is Charged").unwrap_or(false);
        let state = if charged || percent == Some(100) {
            BatteryState::Full
        } else if charging {
            BatteryState::Charging
        } else if power == PowerSource::Battery {
            BatteryState::Discharging
        } else {
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
        return Detection::unsupported(
            "This machine reports no internal battery; macOS desktops have none",
        );
    }
    Detection::available(BatteryInfo { units, power }, source)
}

// --- Graphics adapters ---------------------------------------------------

/// IOKit objects are Mach ports; libc does not name `io_object_t`.
type IoObject = libc::mach_port_t;

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

/// kIOMainPortDefault
const IO_MAIN_PORT: libc::mach_port_t = 0;

/// One string property of an IOKit registry entry. The returned value is an
/// owned copy and is released here.
fn registry_string(entry: IoObject, key: &CStr) -> Option<String> {
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
    let mut buffer = [0_i8; 128];
    // SAFETY: the buffer size matches the capacity passed; the model property is
    // a CFString. A non-zero return means it fitted.
    let ok = unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr(),
            buffer.len() as libc::c_long,
            CF_UTF8,
        )
    };
    // SAFETY: the property reference is an owned copy and is released once.
    unsafe { CFRelease(value) };
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

/// Every IOAccelerator, which is how macOS exposes the graphics adapters.
fn gpu() -> Detection<GpuInfo> {
    let source = "IOKit IOAccelerator";
    // SAFETY: IOServiceMatching returns a dictionary owned by the matching
    // machinery, consumed by IOServiceGetMatchingServices.
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
            id: (!class.is_empty()).then(|| class.clone()),
            driver: None,
            kind,
        });
    }
    // SAFETY: the iterator is an owned object and is released exactly once.
    unsafe { IOObjectRelease(iterator) };
    if devices.is_empty() {
        return Detection::unavailable("No IOAccelerator service is registered");
    }
    Detection::available(GpuInfo { devices }, source)
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

fn sysctl_string(name: &CStr) -> Detection<String> {
    let source = format!("sysctl {}", name.to_string_lossy());
    let mut buffer = [0_u8; 1024];
    let mut size = buffer.len();
    // SAFETY: name is NUL-terminated; buffer and length are valid writable objects.
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        return Detection::error(source, ErrorCode::ApiFailed, io::Error::last_os_error());
    }
    let end = buffer[..size].iter().position(|b| *b == 0).unwrap_or(size);
    match std::str::from_utf8(&buffer[..end]) {
        Ok(value) if !value.trim().is_empty() => {
            Detection::available(value.trim().to_owned(), source)
        }
        _ => Detection::error(
            source,
            ErrorCode::InvalidData,
            "Empty or non-UTF-8 sysctl string",
        ),
    }
}

fn uptime() -> Detection<UptimeInfo> {
    let mut boot = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut size = mem::size_of_val(&boot);
    // SAFETY: sysctl name and timeval buffer are valid and correctly aligned.
    let result = unsafe {
        libc::sysctlbyname(
            c"kern.boottime".as_ptr(),
            (&mut boot as *mut libc::timeval).cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        return Detection::error(
            "sysctl kern.boottime",
            ErrorCode::ApiFailed,
            io::Error::last_os_error(),
        );
    }
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|now| {
            u64::try_from(boot.tv_sec)
                .ok()
                .filter(|boot| *boot > 0)
                .and_then(|boot| now.as_secs().checked_sub(boot))
        });
    match seconds {
        Some(seconds) if size == mem::size_of_val(&boot) => Detection::available(
            UptimeInfo {
                seconds,
                scope: Scope::OperatingSystem,
            },
            "kern.boottime and system clock",
        ),
        _ => Detection::error(
            "kern.boottime",
            ErrorCode::InvalidData,
            "Boot timestamp or system clock is inconsistent",
        ),
    }
}

#[allow(deprecated)]
fn memory() -> Detection<MemoryInfo> {
    let source = "sysctl hw.memsize + Mach host_statistics64";
    let mut total = 0_u64;
    let mut size = mem::size_of_val(&total);
    // SAFETY: hw.memsize writes a u64 into the aligned writable buffer.
    let result = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&mut total as *mut u64).cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        return Detection::error(source, ErrorCode::ApiFailed, io::Error::last_os_error());
    }
    // SAFETY: vm_statistics64 is a C struct of integer counters; zero is valid.
    let mut stats: libc::vm_statistics64 = unsafe { mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: returns a send right for the current host; deallocated below.
    let port = unsafe { libc::mach_host_self() };
    // SAFETY: stats/count are aligned and sized according to HOST_VM_INFO64.
    let result = unsafe {
        libc::host_statistics64(
            port,
            libc::HOST_VM_INFO64,
            (&mut stats as *mut libc::vm_statistics64).cast(),
            &mut count,
        )
    };
    let mut page_size: libc::vm_size_t = 0;
    // SAFETY: page_size is an aligned vm_size_t owned by this frame, and the
    // port is still live at this point.
    let sized = unsafe { host_page_size(port, &mut page_size) };
    // SAFETY: release the send right acquired by mach_host_self in our own task.
    let released = unsafe { mach_port_deallocate(libc::mach_task_self(), port) };
    if result != libc::KERN_SUCCESS {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            format!("Mach return code {result}"),
        );
    }
    // A send right we cannot release is a broken ownership result, not something
    // to drop on the floor while reporting the counters as clean.
    if released != libc::KERN_SUCCESS {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            format!("mach_port_deallocate returned {released}"),
        );
    }
    // A page size of zero would report every process as using no memory at all,
    // which passes the total check below and reads as a real figure.
    let host_page = u64::try_from(page_size)
        .ok()
        .filter(|size| *size > 0 && sized == libc::KERN_SUCCESS);
    let Some(host_page) = host_page else {
        return Detection::error(
            source,
            ErrorCode::ApiFailed,
            format!("host_page_size returned {sized} with page size {page_size}"),
        );
    };
    let used = u64::from(stats.internal_page_count)
        .checked_sub(u64::from(stats.purgeable_count))
        .and_then(|n| n.checked_add(u64::from(stats.wire_count)))
        .and_then(|n| n.checked_add(u64::from(stats.compressor_page_count)))
        .and_then(|n| n.checked_mul(host_page));
    match used {
        Some(used_bytes)
            if total > 0 && used_bytes <= total && size == mem::size_of_val(&total) =>
        {
            Detection::available(
                MemoryInfo {
                    total_bytes: total,
                    used_bytes,
                    definition: MemoryDefinition::AppWiredCompressed,
                    scope: Scope::OperatingSystem,
                },
                source,
            )
        }
        _ => Detection::error(
            source,
            ErrorCode::InvalidData,
            "Inconsistent physical memory counters",
        ),
    }
}

/// `xsw_usage` as declared in <sys/sysctl.h>. The trailing `encrypted` field is
/// `boolean_t`, which is a 32-bit integer on macOS, not a C99 bool.
#[repr(C)]
#[derive(Default)]
struct XswUsage {
    total: u64,
    avail: u64,
    used: u64,
    page_size: u32,
    encrypted: u32,
}

fn swap() -> Detection<SwapInfo> {
    let source = "sysctl vm.swapusage";
    let mut usage = XswUsage::default();
    let mut size = std::mem::size_of_val(&usage);
    // SAFETY: sysctlbyname writes xsw_usage into the aligned writable buffer.
    let result = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            (&mut usage as *mut XswUsage).cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        return Detection::error(source, ErrorCode::ApiFailed, io::Error::last_os_error());
    }
    if size < std::mem::size_of::<u64>() * 3 || usage.used > usage.total {
        return Detection::error(source, ErrorCode::InvalidData, "Inconsistent swap counters");
    }
    Detection::available(
        SwapInfo {
            total_bytes: usage.total,
            used_bytes: usage.used,
        },
        source,
    )
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
        battery: if_selected(selection, ModuleId::Battery, battery),
        local_ip: if_selected(selection, ModuleId::LocalIp, local_ip),
    }
}
