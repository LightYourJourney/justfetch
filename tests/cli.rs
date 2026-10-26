use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run justfetch")
}

#[test]
fn help_and_version() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("--format"));
    assert!(text.contains("--no-color"));
    assert!(help.stderr.is_empty());
    let version = run(&["--version"]);
    assert!(version.status.success());
    // Taken from the manifest so a version bump cannot leave this asserting an
    // old string.
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("justfetch {}\n", env!("CARGO_PKG_VERSION"))
    );
}

/// Every field the snapshot promises, so adding one cannot silently ship
/// without a status.
const FIELDS: [&str; 11] = [
    "os", "host", "kernel", "uptime", "cpu", "memory", "swap", "disk", "shell", "terminal",
    "local_ip",
];

#[test]
fn invalid_arguments_are_stderr_only() {
    for args in [
        &["--format", "xml"][..],
        &["--logo", "bogus"],
        &["--no-such-option"],
        &["--format"],
    ] {
        let out = run(args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}

#[test]
fn json_is_one_structured_document() {
    let out = run(&["--format", "json"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert!(!out.stdout.contains(&27));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    const KNOWN: [&str; 5] = [
        "available",
        "unavailable",
        "unsupported",
        "redacted",
        "error",
    ];
    for field in FIELDS {
        let status = json[field]["status"]
            .as_str()
            .unwrap_or_else(|| panic!("missing status for {field}"));
        assert!(
            KNOWN.contains(&status),
            "{field} has unknown status {status}"
        );
        if status == "available" {
            assert!(
                json[field]["source"].is_string(),
                "{field} is available without a source"
            );
            let confidence = json[field]["confidence"].as_str().unwrap();
            assert!(
                confidence == "confirmed" || confidence == "estimated",
                "{field} has unknown confidence {confidence}"
            );
        }
    }
    if json["memory"]["status"] == "available" {
        let memory = &json["memory"]["value"];
        assert!(memory["total_bytes"].as_u64().unwrap() > 0);
        assert!(memory["used_bytes"].as_u64().unwrap() <= memory["total_bytes"].as_u64().unwrap());
    }
}

#[test]
fn plain_text_and_no_logo() {
    let out = run(&["--format", "text", "--no-color", "--logo", "none"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert!(!out.stdout.contains(&27));
    let text = String::from_utf8(out.stdout).unwrap();
    for label in [
        "OS", "Host", "Kernel", "Uptime", "CPU", "Memory", "Swap", "Disk", "Shell", "Term", "IP",
    ] {
        assert!(
            text.lines().any(|l| l.starts_with(label)),
            "{label}: {text}"
        );
    }
}

#[test]
fn privacy_withholds_identity_and_addresses() {
    let out = run(&["--privacy", "--format", "json"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["identity"]["username"]["status"], "redacted");
    assert_eq!(value["identity"]["hostname"]["status"], "redacted");
    assert_eq!(value["local_ip"]["status"], "redacted");
    // Redaction withholds values; it does not change the schema.
    assert!(value["identity"]["username"].get("value").is_none());
    assert_eq!(value["schema_version"], 1);

    let text = run(&["--privacy", "--logo", "none", "--no-color"]);
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert_eq!(text.lines().next(), Some("redacted@redacted"), "{text}");
    assert!(text.contains("IP      redacted"), "{text}");
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        assert!(
            !text.contains(&format!("{user}@")),
            "user name leaked: {text}"
        );
    }
}

#[test]
fn default_and_dumb_terminal_are_plain_when_redirected() {
    let out = run(&[]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert!(!out.stdout.contains(&27));
    assert!(String::from_utf8(out.stdout).unwrap().contains("Memory"));
    let out = Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .env("TERM", "dumb")
        .env("LC_ALL", "C")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stdout.is_ascii());
    assert!(!out.stdout.contains(&27));
}

#[test]
fn json_ignores_visual_flags_and_environment() {
    let out = run(&["--format", "json", "--logo", "none", "--no-color"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["identity"]["hostname"].is_object());
    assert!(!out.stdout.contains(&27));
}
