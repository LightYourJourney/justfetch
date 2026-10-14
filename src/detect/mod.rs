//! Collection orchestrator; detection never writes to stdout or stderr.
pub mod linux;
pub mod shell;

use crate::{model::*, platform};
use sysinfo::{CpuRefreshKind, System};

pub fn collect() -> SystemSnapshot {
    let data = platform::collect();
    let mut system = System::new();
    system.refresh_cpu_list(CpuRefreshKind::nothing());
    let cpu = match (
        system
            .cpus()
            .first()
            .filter(|cpu| !cpu.brand().trim().is_empty()),
        crate::platform::logical_cpu_count(system.cpus().len()),
    ) {
        (Some(cpu), Ok(logical_cores)) => Detection::available(
            CpuInfo {
                name: cpu.brand().trim().to_owned(),
                logical_cores,
            },
            if cfg!(windows) {
                "sysinfo CPU brand + GetActiveProcessorCount(all groups)"
            } else {
                "sysinfo CPU native adapter"
            },
        ),
        (_, Err(error)) => Detection::error("logical processor count", ErrorCode::ApiFailed, error),
        (None, _) => Detection::unavailable("CPU brand not exposed by the system adapter"),
    };
    let shell = shell::collect();
    let variable = if cfg!(windows) { "USERNAME" } else { "USER" };
    let username = match std::env::var(variable)
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        Some(name) => Detection::estimated(
            name,
            variable,
            "Environment identity; not an authenticated user lookup",
        ),
        None => Detection::unavailable("User environment variable is absent or not Unicode"),
    };
    SystemSnapshot {
        schema_version: 1,
        identity: Identity {
            username,
            hostname: data.hostname,
        },
        environment: data.environment,
        os: data.os,
        host: data.host,
        kernel: data.kernel,
        uptime: data.uptime,
        cpu,
        memory: data.memory,
        shell,
    }
}
