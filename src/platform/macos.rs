use super::{PlatformData, uname, uname_identity};
use crate::model::*;
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
    // SAFETY: sysconf has no pointer arguments or side effects on process memory.
    let page_size = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok();
    let used = u64::from(stats.internal_page_count)
        .checked_sub(u64::from(stats.purgeable_count))
        .and_then(|n| n.checked_add(u64::from(stats.wire_count)))
        .and_then(|n| n.checked_add(u64::from(stats.compressor_page_count)))
        .and_then(|n| page_size.and_then(|page| n.checked_mul(page)));
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

pub fn collect() -> PlatformData {
    let data = uname();
    let (hostname, kernel) = uname_identity(&data);
    let os = sysctl_string(c"kern.osproductversion").map(|version| OsInfo {
        family: OsFamily::Macos,
        name: "macOS".to_owned(),
        version: Some(version),
        id: Some("macos".to_owned()),
    });
    let host = sysctl_string(c"hw.model").map(|name| HostInfo {
        name,
        kind: HostKind::DeviceModel,
    });
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel,
        uptime: uptime(),
        memory: memory(),
    }
}
