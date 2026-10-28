//! Collection orchestrator; detection never writes to stdout or stderr.
pub mod linux;
pub mod shell;

use crate::{
    model::*,
    module::{ModuleId, if_selected},
    platform,
};
use sysinfo::{CpuRefreshKind, System};

/// The terminal application, which is not the shell. `TERM_PROGRAM` and
/// `WT_SESSION` identify the program; `TERM` does not, and the two are reported
/// as different kinds rather than merged. None of them requires a TTY, so
/// redirected output reports the same environment.
fn terminal() -> Detection<TerminalInfo> {
    let text = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    if let Some(name) = text("TERM_PROGRAM") {
        return Detection::available(
            TerminalInfo {
                name,
                version: text("TERM_PROGRAM_VERSION"),
                kind: TerminalKind::Application,
            },
            "TERM_PROGRAM",
        );
    }
    // An empty value is not evidence of Windows Terminal, so it goes through
    // the same non-empty filter as the others.
    if text("WT_SESSION").is_some() {
        return Detection::available(
            TerminalInfo {
                name: "Windows Terminal".to_owned(),
                version: None,
                kind: TerminalKind::Application,
            },
            "WT_SESSION",
        );
    }
    // TERM names a capability set, not a program. Reporting it as the terminal
    // would answer a question it does not answer, so it is labelled as what it
    // is and marked as an environment hint rather than a confirmed fact.
    match text("TERM") {
        Some(name) => Detection::estimated(
            TerminalInfo {
                name,
                version: None,
                kind: TerminalKind::CapabilityType,
            },
            "TERM",
            "TERM names a capability type, not the terminal application",
        ),
        None => Detection::unavailable(
            "Neither TERM_PROGRAM, WT_SESSION nor TERM is set, so the terminal is unknown",
        ),
    }
}

/// Collect only what `selection` asks for. Anything else is reported as
/// deselected, and its collector is never called.
pub fn collect(selection: &[ModuleId]) -> SystemSnapshot {
    let data = platform::collect(selection);
    // Enumerating processors is not free, so it happens only for a CPU row.
    let cpu = if_selected(selection, ModuleId::Cpu, || {
        let mut system = System::new();
        system.refresh_cpu_list(CpuRefreshKind::nothing());
        match (
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
            (_, Err(error)) => {
                Detection::error("logical processor count", ErrorCode::ApiFailed, error)
            }
            (None, _) => Detection::unavailable("CPU brand not exposed by the system adapter"),
        }
    });
    let shell = if_selected(selection, ModuleId::Shell, shell::collect);
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
        swap: data.swap,
        disk: data.disk,
        gpu: data.gpu,
        display: data.display,
        battery: data.battery,
        shell,
        terminal: if_selected(selection, ModuleId::Terminal, terminal),
        local_ip: data.local_ip,
    }
}
