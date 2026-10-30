//! Collection orchestrator; detection never writes to stdout or stderr.
pub mod desktop;
pub mod hardware;
pub mod linux;
pub mod locale;
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

/// Turn what the CPU adapter reported into a detection.
///
/// `brand` is `None` when the adapter has no name — on ARM64 Linux the kernel
/// exposes `CPU part: 0x000` for a generic virtual CPU, which sysinfo cannot map
/// to a name, and `/proc/cpuinfo` carries no `model name` either. The module is
/// then `unavailable` and the known core count is discarded, because
/// `CpuInfo.name` is required by the v1 JSON contract. Preserving the count
/// would mean making the name optional, which is a v1.1 schema decision.
fn cpu_detection(brand: Option<&str>, cores: std::io::Result<usize>) -> Detection<CpuInfo> {
    let name = brand.map(str::trim).filter(|name| !name.is_empty());
    match (name, cores) {
        (Some(name), Ok(logical_cores)) => Detection::available(
            CpuInfo {
                name: name.to_owned(),
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
        // `cpu_detection` trims, so the adapter's brand is passed as-is.
        cpu_detection(
            system.cpus().first().map(|cpu| cpu.brand()),
            crate::platform::logical_cpu_count(system.cpus().len()),
        )
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
        packages: data.packages,
        desktop: data.desktop,
        locale: data.locale,
        battery: data.battery,
        shell,
        terminal: if_selected(selection, ModuleId::Terminal, terminal),
        local_ip: data.local_ip,
    }
}

#[cfg(test)]
mod cpu_tests {
    use super::*;

    /// The ARM64 Linux virtual machine case: the kernel reports `CPU part: 0x000`,
    /// sysinfo has no name to map it to and `/proc/cpuinfo` has no `model name`,
    /// so the adapter yields an empty brand while the core count is known.
    ///
    /// This pins the v1.0 behaviour — `unavailable`, cores discarded — so a v1.1
    /// change to keep the count has to change this test on purpose rather than
    /// land by accident.
    #[test]
    fn an_unnamed_cpu_is_unavailable_even_with_a_known_core_count() {
        let detection = cpu_detection(None, Ok(4));
        assert_eq!(
            detection,
            Detection::unavailable("CPU brand not exposed by the system adapter")
        );
        let detection = cpu_detection(Some("   "), Ok(4));
        assert_eq!(
            detection,
            Detection::unavailable("CPU brand not exposed by the system adapter")
        );
    }

    #[test]
    fn a_named_cpu_reports_both_fields() {
        // The values, not the source string: restating the source expression
        // here would only assert that the test copies the implementation.
        let detection = cpu_detection(Some("  Apple M3  "), Ok(8));
        let Detection::Available {
            value, confidence, ..
        } = detection
        else {
            panic!("a named CPU with a core count should be available");
        };
        assert_eq!(value.name, "Apple M3", "the brand is trimmed");
        assert_eq!(value.logical_cores, 8);
        assert_eq!(confidence, Confidence::Confirmed);
    }

    #[test]
    fn a_failed_core_count_is_an_error_not_an_absence() {
        let detection = cpu_detection(
            Some("Apple M3"),
            Err(std::io::Error::other("no processors reported")),
        );
        assert!(
            matches!(detection, Detection::Error { .. }),
            "{detection:?}"
        );
    }
}
