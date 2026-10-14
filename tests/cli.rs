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
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        "justfetch 0.1.0\n"
    );
}

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
    for field in ["os", "host", "kernel", "uptime", "cpu", "memory", "shell"] {
        assert!(
            json[field]["status"].is_string(),
            "missing status for {field}"
        );
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
    for field in ["OS", "Host", "Kernel", "Uptime", "CPU", "Memory", "Shell"] {
        assert!(
            text.lines().any(|l| l.starts_with(field)),
            "{field}: {text}"
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
