//! Semantic assertions about what the detector reports on the host running the
//! test. These run on every platform in CI, so a collector that silently starts
//! failing breaks the build instead of passing a smoke step that only proves
//! the process started and exited.

use std::process::Command;

fn snapshot() -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .args(["--format", "json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run justfetch");
    assert!(output.status.success(), "exit status {:?}", output.status);
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("exactly one JSON document")
}

fn status<'a>(json: &'a serde_json::Value, field: &str) -> &'a str {
    json[field]["status"]
        .as_str()
        .unwrap_or_else(|| panic!("{field} has no status: {json}"))
}

#[test]
fn core_fields_are_available_on_a_real_host() {
    let json = snapshot();
    assert_eq!(json["schema_version"], 1);
    // These depend only on the kernel and the C library, so a runner that
    // cannot answer them is a regression worth failing on.
    for field in ["os", "host", "kernel", "uptime", "cpu", "memory", "disk"] {
        assert_eq!(status(&json, field), "available", "{field}: {json}");
    }
    // Shell is deliberately not in that list: with no recognized parent and no
    // SHELL/ComSpec exported it is legitimately unavailable, and this test does
    // not control the environment it runs in.
    match status(&json, "shell") {
        "available" => {
            let kind = json["shell"]["value"]["kind"].as_str().unwrap();
            assert!(
                kind == "current_parent" || kind == "configured_default",
                "{json}"
            );
        }
        "unavailable" => assert!(json["shell"]["reason"].is_string(), "{json}"),
        other => panic!("unexpected shell status {other}: {json}"),
    }
}

#[test]
fn reported_numbers_are_self_consistent() {
    let json = snapshot();
    assert_eq!(status(&json, "memory"), "available", "{json}");
    assert_eq!(status(&json, "disk"), "available", "{json}");
    let memory = &json["memory"]["value"];
    let total = memory["total_bytes"].as_u64().expect("memory total");
    let used = memory["used_bytes"].as_u64().expect("memory used");
    assert!(total > 0, "{json}");
    assert!(used <= total, "used {used} exceeds total {total}");

    let disk = &json["disk"]["value"];
    let disk_total = disk["total_bytes"].as_u64().expect("disk total");
    assert!(disk_total > 0, "{json}");
    assert!(
        disk["used_bytes"].as_u64().expect("disk used") <= disk_total,
        "{json}"
    );
    assert!(!disk["path"].as_str().expect("disk path").is_empty());

    if status(&json, "swap") == "available" {
        let swap = &json["swap"]["value"];
        assert!(
            swap["used_bytes"].as_u64().expect("swap used")
                <= swap["total_bytes"].as_u64().expect("swap total"),
            "{json}"
        );
    }

    assert!(json["cpu"]["value"]["logical_cores"].as_u64().unwrap() >= 1);
    assert!(json["uptime"]["value"]["seconds"].as_u64().is_some());
}

#[test]
fn memory_agrees_with_an_independent_reader() {
    // A four-fold undercount from pairing Mach counters with the wrong page
    // size satisfies every `used <= total` check, which is exactly how that
    // defect survived four green CI jobs. Comparing against a second reader is
    // what catches it. The bound is loose on purpose: the two use different
    // definitions of "used".
    let json = snapshot();
    let used = json["memory"]["value"]["used_bytes"]
        .as_u64()
        .expect("memory used_bytes");
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let independent = system.used_memory();
    assert!(independent > 0, "independent reader saw no used memory");
    assert!(
        used <= independent.saturating_mul(2) && independent <= used.saturating_mul(2),
        "justfetch reports {used} bytes used; an independent reader says {independent}"
    );
}

#[test]
fn listed_addresses_are_routable() {
    let json = snapshot();
    if status(&json, "local_ip") != "available" {
        return;
    }
    let addresses = json["local_ip"]["value"]["addresses"]
        .as_array()
        .expect("address list");
    assert!(!addresses.is_empty(), "{json}");
    for entry in addresses {
        let address: std::net::Ipv4Addr = entry["address"]
            .as_str()
            .expect("address string")
            .parse()
            .expect("IPv4 address");
        assert!(
            !address.is_loopback() && !address.is_link_local() && !address.is_unspecified(),
            "non-routable address listed: {json}"
        );
    }
}

#[test]
fn terminal_never_claims_an_application_it_cannot_name() {
    let json = snapshot();
    if status(&json, "terminal") != "available" {
        return;
    }
    let confidence = json["terminal"]["confidence"].as_str().unwrap();
    match json["terminal"]["value"]["kind"].as_str().unwrap() {
        "application" => assert_eq!(confidence, "confirmed", "{json}"),
        // A capability type is an environment hint, never a confirmed application.
        "capability_type" => {
            assert_eq!(confidence, "estimated", "{json}");
            assert!(json["terminal"]["note"].is_string(), "{json}");
        }
        other => panic!("unknown terminal kind {other}: {json}"),
    }
}

#[test]
fn ansi_support_follows_the_platform_console() {
    // Under `cargo test` stdout is a pipe. Unix reports the capability
    // unconditionally; Windows has to ask the console and must not assume it,
    // which is the behaviour this pins.
    assert_eq!(justfetch::platform::supports_ansi(), cfg!(unix));
}

#[test]
fn windows_does_not_fabricate_a_swap_figure() {
    // The documented contract: Windows exposes a commit limit, not page-file
    // usage, and no permitted API supplies the latter. Change this deliberately
    // if that ever stops being true.
    if !cfg!(windows) {
        return;
    }
    let json = snapshot();
    assert_eq!(status(&json, "swap"), "unsupported", "{json}");
}
