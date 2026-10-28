use std::process::{Command, Output};

/// A directory that is never created, so the binary looks for a user
/// configuration and finds none — whatever the person running the suite has in
/// their real one.
fn isolated_config_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("jf-cli-suite-has-no-config-here")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .args(args)
        .env("NO_COLOR", "1")
        .env("XDG_CONFIG_HOME", isolated_config_dir())
        .env("APPDATA", isolated_config_dir())
        .output()
        .expect("run justfetch")
}

/// A path as a TOML basic string. A backslash starts an escape there, and a
/// Windows path is full of them; this is the same rule the documentation gives
/// users.
fn toml_path(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

/// Whether a row with this label is drawn. Checking the start of a line, not
/// the whole output: a value may mention the word — a CPU model can contain
/// "CPU " — and that says nothing about which label was used.
fn has_row(text: &str, label: &str) -> bool {
    text.lines().any(|line| line.starts_with(label))
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("jf-cli-test-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

#[test]
fn modules_select_and_order_the_rows() {
    let out = run(&["--modules", "memory,cpu", "--logo", "none", "--no-color"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let text = String::from_utf8(out.stdout).unwrap();
    let memory = text.find("Memory").expect("a Memory row");
    let cpu = text.find("CPU").expect("a CPU row");
    assert!(memory < cpu, "rows follow the requested order:\n{text}");
    for absent in [
        "OS ", "Host", "Kernel", "Uptime", "Swap", "Disk", "Shell", "Term", "IP ",
    ] {
        assert!(
            !text.lines().any(|line| line.starts_with(absent)),
            "{absent} was not selected but was drawn:\n{text}"
        );
    }
}

#[test]
fn json_marks_unselected_modules_as_deselected() {
    let out = run(&["--format", "json", "--modules", "cpu"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["cpu"]["status"], "available");
    for field in [
        "os", "host", "kernel", "uptime", "memory", "swap", "disk", "shell", "terminal", "local_ip",
    ] {
        assert_eq!(json[field]["status"], "deselected", "{field}");
        assert!(json[field].get("value").is_none(), "{field} kept a value");
    }
    // Identity is not a module: it stays for the title line.
    assert!(json["identity"]["hostname"].is_object());
}

#[test]
fn a_config_file_sets_modules_labels_and_separator() {
    let dir = temp_dir("config");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "modules = [\"cpu\"]\n\n[layout]\nlogo = \"none\"\nseparator = \" :: \"\n\n[labels]\ncpu = \"Processor\"\n",
    )
    .unwrap();
    let out = run(&["--config", path.to_str().unwrap(), "--no-color"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Processor :: "), "{text}");
    assert!(!has_row(&text, "CPU "), "{text}");
}

#[test]
fn the_command_line_overrides_the_config_file() {
    let dir = temp_dir("precedence");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "modules = [\"cpu\"]\n\n[layout]\nlogo = \"auto\"\n\n[color]\nenabled = false\n",
    )
    .unwrap();
    let out = run(&[
        "--config",
        path.to_str().unwrap(),
        "--modules",
        "os",
        "--logo",
        "none",
        "--no-color",
    ]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(has_row(&text, "OS "), "{text}");
    assert!(
        !has_row(&text, "CPU "),
        "the file's module list should be overridden:\n{text}"
    );
}

#[test]
fn init_writes_a_usable_file_and_refuses_to_overwrite() {
    let dir = temp_dir("init");
    let path = dir.join("config.toml");
    let first = run(&["--config", path.to_str().unwrap(), "init"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(path.exists());
    // What init writes must itself be a valid configuration.
    let used = run(&[
        "--config",
        path.to_str().unwrap(),
        "--no-color",
        "--logo",
        "none",
    ]);
    assert!(
        used.status.success(),
        "{}",
        String::from_utf8_lossy(&used.stderr)
    );
    let second = run(&["--config", path.to_str().unwrap(), "init"]);
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
}

#[test]
fn doctor_distinguishes_a_good_configuration_from_a_broken_one() {
    let dir = temp_dir("doctor");
    let good = dir.join("good.toml");
    std::fs::write(&good, "modules = [\"cpu\"]\n").unwrap();
    let ok = run(&["--config", good.to_str().unwrap(), "doctor"]);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );
    assert!(String::from_utf8_lossy(&ok.stdout).contains("no problems found"));

    let bad = dir.join("bad.toml");
    std::fs::write(&bad, "modules = [\"gpu\"]\n").unwrap();
    let broken = run(&["--config", bad.to_str().unwrap(), "doctor"]);
    assert!(!broken.status.success());
    let report = String::from_utf8_lossy(&broken.stdout);
    assert!(report.contains("problem:"), "{report}");
    assert!(report.contains("not a module id"), "{report}");
}

#[test]
fn doctor_reports_a_missing_config_file_once() {
    let dir = temp_dir("doctor-missing");
    let missing = dir.join("absent.toml");
    let output = run(&["--config", missing.to_str().unwrap(), "doctor"]);
    assert!(!output.status.success());
    let report = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        report.matches("file not found").count(),
        1,
        "the same missing file is reported twice:\n{report}"
    );
}

#[test]
fn configuration_errors_reach_stderr_without_stdout() {
    for (args, expected) in [
        (&["--modules", "gpu"][..], "not a module id"),
        (&["--modules", "os,os"], "listed twice"),
        (
            &["--config", "/nonexistent/justfetch.toml"],
            "file not found",
        ),
    ] {
        let out = run(args);
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
    }
}

#[test]
fn privacy_does_not_turn_deselection_into_redaction() {
    // The two are different facts: the identity is withheld, the address was
    // never collected.
    let out = run(&["--format", "json", "--privacy", "--modules", "cpu"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["identity"]["username"]["status"], "redacted");
    assert_eq!(json["identity"]["hostname"]["status"], "redacted");
    assert_eq!(json["local_ip"]["status"], "deselected");
    assert_eq!(json["host"]["status"], "deselected");
}

#[test]
fn config_is_accepted_after_a_subcommand() {
    let dir = temp_dir("subcommand-config");
    let path = dir.join("config.toml");
    let out = run(&["init", "--config", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(path.exists());
}

#[test]
fn the_help_lists_every_module() {
    // The list in `--help` is hand-written, so it is checked against the
    // canonical set instead of being trusted to stay in step.
    let help = run(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for module in justfetch::module::ModuleId::ALL {
        assert!(
            text.contains(module.id()),
            "{} is missing from --help",
            module.id()
        );
    }
}

#[test]
fn a_unicode_config_path_works() {
    // Item 6 of the round's specification names Unicode paths; the config path
    // is not ASCII in every home directory.
    let dir = temp_dir("unicode");
    let nested = dir.join("配置 目录");
    std::fs::create_dir_all(&nested).unwrap();
    let path = nested.join("config.toml");
    std::fs::write(&path, "modules = [\"cpu\"]\n\n[layout]\nlogo = \"none\"\n").unwrap();
    let out = run(&["--config", path.to_str().unwrap(), "--no-color"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("CPU"));
}

#[test]
fn themes_lists_every_built_in_theme() {
    let out = run(&["themes"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    for name in justfetch::render::theme::names() {
        assert!(text.contains(name), "`themes` omitted {name}:\n{text}");
    }
    assert!(
        !text.contains('\u{1b}'),
        "a redirected listing must stay plain"
    );
}

#[test]
fn an_unknown_theme_layout_or_logo_names_the_choices() {
    for (flag, value, expected) in [
        ("--theme", "solarized", "expected one of"),
        ("--layout", "columns", "expected one of"),
        ("--logo", "penguin", "expected `auto`, `none`"),
    ] {
        let out = run(&[flag, value, "--no-color"]);
        assert!(!out.status.success(), "{flag} {value} was accepted");
        assert!(out.stdout.is_empty(), "{flag} {value} wrote to stdout");
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(error.contains(expected), "{flag} {value}: {error}");
        assert!(
            error.contains(value),
            "{flag} {value} was not named: {error}"
        );
    }
}

#[test]
fn a_theme_from_the_file_is_overridden_by_the_command_line() {
    let dir = temp_dir("theme-precedence");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "[theme]\nname = \"nord\"\n[layout]\nname = \"compact\"\n",
    )
    .unwrap();
    let from_file = run(&["--config", path.to_str().unwrap(), "doctor"]);
    assert!(from_file.status.success());
    let report = String::from_utf8_lossy(&from_file.stdout);
    assert!(report.contains("theme nord"), "{report}");
    assert!(report.contains("compact layout"), "{report}");

    let overridden = run(&[
        "--config",
        path.to_str().unwrap(),
        "--theme",
        "gruvbox",
        "--layout",
        "vertical",
        "doctor",
    ]);
    assert!(overridden.status.success());
    let report = String::from_utf8_lossy(&overridden.stdout);
    assert!(report.contains("theme gruvbox"), "{report}");
    assert!(report.contains("vertical layout"), "{report}");
}

#[test]
fn a_custom_theme_file_is_used_and_a_bad_colour_is_refused() {
    let dir = temp_dir("custom-theme");
    let theme = dir.join("mine.toml");
    std::fs::write(&theme, "name = \"mine\"\ntitle = \"#aabbcc\"\n").unwrap();
    let config = dir.join("config.toml");
    std::fs::write(
        &config,
        format!("[theme]\nfile = \"{}\"\n", toml_path(&theme)),
    )
    .unwrap();
    let out = run(&["--config", config.to_str().unwrap(), "doctor"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("theme mine"));

    std::fs::write(&theme, "name = \"mine\"\ntitle = \"chartreuse\"\n").unwrap();
    let bad = run(&["--config", config.to_str().unwrap(), "doctor"]);
    assert!(!bad.status.success());
    let error = format!(
        "{}{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    assert!(error.contains("title"), "{error}");
    assert!(error.contains("chartreuse"), "{error}");
}

#[test]
fn a_custom_logo_file_is_drawn_and_a_malformed_one_is_refused() {
    let dir = temp_dir("custom-logo");
    let logo = dir.join("mine.txt");
    std::fs::write(&logo, "  ____\n / AA \\\n \\____/\n").unwrap();
    let out = run(&[
        "--logo-file",
        logo.to_str().unwrap(),
        "--no-color",
        "--modules",
        "os",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("AA"), "{text}");

    let empty = dir.join("empty.txt");
    std::fs::write(&empty, "\n\n").unwrap();
    let bad = run(&["--logo-file", empty.to_str().unwrap(), "--no-color"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("empty"));
}

#[test]
fn json_is_unaffected_by_a_theme_or_layout() {
    let out = run(&[
        "--format", "json", "--theme", "nord", "--layout", "vertical",
    ]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert!(!out.stdout.contains(&27));
}

#[test]
fn the_help_lists_every_theme_layout_and_logo() {
    let help = String::from_utf8(run(&["--help"]).stdout).unwrap();
    for name in justfetch::render::theme::names() {
        assert!(help.contains(name), "--help omits the theme {name}");
    }
    for layout in justfetch::render::layout::Layout::ALL {
        assert!(
            help.contains(layout.id()),
            "--help omits the layout {layout}"
        );
    }
    for logo in justfetch::render::logo::built_in_names() {
        assert!(help.contains(logo), "--help omits the logo {logo}");
    }
}

#[test]
fn a_theme_file_can_be_chosen_from_the_command_line() {
    let dir = temp_dir("theme-file-flag");
    let theme = dir.join("mine.toml");
    std::fs::write(&theme, "name = \"mine\"\ntitle = \"blue\"\n").unwrap();

    let out = run(&["--theme-file", theme.to_str().unwrap(), "doctor"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("theme mine"));

    let conflict = run(&[
        "--theme",
        "nord",
        "--theme-file",
        theme.to_str().unwrap(),
        "doctor",
    ]);
    assert!(!conflict.status.success());
    // `doctor` reports a configuration error on stdout, as a problem line.
    assert!(
        String::from_utf8_lossy(&conflict.stdout).contains("one or the other"),
        "{}",
        String::from_utf8_lossy(&conflict.stdout)
    );
}

#[test]
fn asking_for_two_logos_on_one_command_line_is_refused() {
    let dir = temp_dir("logo-conflict");
    let art = dir.join("art.txt");
    std::fs::write(&art, " ART\n").unwrap();
    for value in ["none", "arch"] {
        let out = run(&["--logo", value, "--logo-file", art.to_str().unwrap()]);
        assert!(
            !out.status.success(),
            "--logo {value} plus --logo-file was accepted"
        );
        assert!(String::from_utf8_lossy(&out.stderr).contains("one or the other"));
    }
}

#[test]
fn a_newline_in_a_configuration_value_cannot_forge_a_line() {
    let dir = temp_dir("newline-injection");
    let path = dir.join("config.toml");
    std::fs::write(&path, "modules = [\"evil\\u000Aproblem: forged\"]\n").unwrap();
    let out = run(&["--config", path.to_str().unwrap(), "doctor"]);
    assert!(!out.status.success());
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !report.lines().any(|line| line.trim() == "problem: forged"),
        "a value forged a diagnostic line:\n{report}"
    );
    assert!(report.contains("is not a module id"), "{report}");
}

#[test]
fn asking_for_json_from_themes_returns_json() {
    let out = run(&["--format", "json", "themes"]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let list = value.as_array().expect("an array of themes");
    assert_eq!(list.len(), justfetch::render::theme::names().len());
    for entry in list {
        assert!(entry["name"].is_string());
        assert!(entry["description"].is_string());
        assert!(entry["swatches"].is_array());
    }
}

#[test]
fn a_windows_style_path_is_escaped_for_toml() {
    assert_eq!(
        toml_path(std::path::Path::new(r"C:\Users\a\b.toml")),
        r"C:\\Users\\a\\b.toml"
    );
}

#[cfg(unix)]
#[test]
fn a_newline_in_a_file_path_cannot_forge_a_line() {
    let dir = temp_dir("path-newline");
    // A Unix filename may contain a newline, and the parser's message embeds
    // the path. It must not become a second line of output.
    let theme = dir.join("bad\nproblem: forged.toml");
    std::fs::write(&theme, "title = \"nope\"\n").unwrap();
    let out = run(&["--theme-file", theme.to_str().unwrap()]);
    assert!(!out.status.success());
    let report = String::from_utf8_lossy(&out.stderr);
    assert!(
        !report
            .lines()
            .any(|line| line.starts_with("problem: forged")),
        "the path forged a line:\n{report}"
    );
    assert!(report.contains("missing field"), "{report}");
}
