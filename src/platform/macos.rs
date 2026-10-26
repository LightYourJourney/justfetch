use super::{PlatformData, disk, local_ip, uname, uname_identity};
use crate::model::*;
use crate::module::{ModuleId, wanted};
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
    let os = if wanted(selection, ModuleId::Os) {
        sysctl_string(c"kern.osproductversion").map(|version| OsInfo {
            family: OsFamily::Macos,
            name: "macOS".to_owned(),
            version: Some(version),
            id: Some("macos".to_owned()),
        })
    } else {
        Detection::deselected()
    };
    let host = if wanted(selection, ModuleId::Host) {
        sysctl_string(c"hw.model").map(|name| HostInfo {
            name,
            kind: HostKind::DeviceModel,
        })
    } else {
        Detection::deselected()
    };
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel: if wanted(selection, ModuleId::Kernel) {
            kernel
        } else {
            Detection::deselected()
        },
        uptime: if wanted(selection, ModuleId::Uptime) {
            uptime()
        } else {
            Detection::deselected()
        },
        memory: if wanted(selection, ModuleId::Memory) {
            memory()
        } else {
            Detection::deselected()
        },
        swap: if wanted(selection, ModuleId::Swap) {
            swap()
        } else {
            Detection::deselected()
        },
        disk: if wanted(selection, ModuleId::Disk) {
            disk()
        } else {
            Detection::deselected()
        },
        local_ip: if wanted(selection, ModuleId::LocalIp) {
            local_ip()
        } else {
            Detection::deselected()
        },
    }
}
