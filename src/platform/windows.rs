use super::PlatformData;
use crate::model::*;
use std::io;
use sysinfo::System;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub fn logical_cpu_count(_detected: usize) -> io::Result<usize> {
    use windows_sys::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};
    // SAFETY: pure system query with the documented all-groups selector.
    let count = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if count == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(count as usize)
    }
}

pub fn parent_executable() -> io::Result<String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::{Diagnostics::ToolHelp::*, Threading::*},
    };
    struct OwnedHandle(HANDLE);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns exactly one validated native handle.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    // One documented ToolHelp snapshot obtains the parent ID. No per-process
    // metadata scan or sysinfo refresh; stop as soon as our own entry is found.
    // SAFETY: read-only snapshot of process IDs; no borrowed pointer arguments.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = OwnedHandle(snapshot);
    // SAFETY: PROCESSENTRY32W is an integer/array C record with valid zero state.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    // SAFETY: validated snapshot handle and correctly sized writable entry.
    if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let own_pid = std::process::id();
    while entry.th32ProcessID != own_pid {
        // SAFETY: entry remains initialized with its size, snapshot is live.
        if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: request only query rights; ID came from a native process snapshot.
    let parent = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            entry.th32ParentProcessID,
        )
    };
    if parent.is_null() {
        return Err(io::Error::last_os_error());
    }
    let parent = OwnedHandle(parent);
    let mut buffer = vec![0_u16; 32768];
    let mut size = buffer.len() as u32;
    // SAFETY: live handle, writable UTF-16 buffer, exact capacity in code units.
    if unsafe { QueryFullProcessImageNameW(parent.0, 0, buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    String::from_utf16(&buffer[..size as usize])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn memory() -> Detection<MemoryInfo> {
    // SAFETY: MEMORYSTATUSEX is an integer-only C struct with valid all-zero state.
    let mut data: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    data.dwLength = std::mem::size_of_val(&data) as u32;
    // SAFETY: data is aligned, writable, and dwLength specifies its exact size.
    if unsafe { GlobalMemoryStatusEx(&mut data) } == 0 {
        return Detection::error(
            "GlobalMemoryStatusEx",
            ErrorCode::ApiFailed,
            std::io::Error::last_os_error(),
        );
    }
    match data.ullTotalPhys.checked_sub(data.ullAvailPhys) {
        Some(used_bytes) if data.ullTotalPhys > 0 => Detection::available(
            MemoryInfo {
                total_bytes: data.ullTotalPhys,
                used_bytes,
                definition: MemoryDefinition::TotalMinusAvailable,
                scope: Scope::OperatingSystem,
            },
            "GlobalMemoryStatusEx",
        ),
        _ => Detection::error(
            "GlobalMemoryStatusEx",
            ErrorCode::InvalidData,
            "Inconsistent physical memory counters",
        ),
    }
}

pub fn collect() -> PlatformData {
    let hostname = match System::host_name().filter(|s| !s.is_empty()) {
        Some(name) => Detection::available(name, "sysinfo GetComputerNameExW"),
        None => Detection::unavailable("Hostname is not exposed by the Windows adapter"),
    };
    let build = System::kernel_version().filter(|s| !s.is_empty());
    let os = match (System::long_os_version(), &build) {
        (Some(name), Some(build)) => Detection::available(
            OsInfo {
                family: OsFamily::Windows,
                name,
                version: Some(format!("build {build}")),
                id: Some("windows".into()),
            },
            "sysinfo Windows version registry",
        ),
        _ => Detection::unavailable("Windows product name or build registry value unavailable"),
    };
    let kernel = match build {
        Some(build) => Detection::available(
            format!("Windows NT build {build}"),
            "sysinfo Windows CurrentBuildNumber",
        ),
        None => Detection::unavailable("Windows kernel build is unavailable"),
    };
    let host = hostname.clone().map(|name| HostInfo {
        name,
        kind: HostKind::Hostname,
    });
    let uptime = Detection::available(
        UptimeInfo {
            seconds: System::uptime(),
            scope: Scope::OperatingSystem,
        },
        "GetTickCount64",
    );
    PlatformData {
        hostname,
        environment: Environment::Native,
        os,
        host,
        kernel,
        uptime,
        memory: memory(),
    }
}
