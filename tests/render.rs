use anstyle::AnsiColor;
use justfetch::{
    detect::shell::recognized_name,
    model::*,
    render::{
        json, layout, logo,
        terminal::{self, Options},
        theme,
    },
};

fn fixture() -> SystemSnapshot {
    SystemSnapshot {
        schema_version: 1,
        identity: Identity {
            username: Detection::available("tester".into(), "fixture"),
            hostname: Detection::available("host".into(), "fixture"),
        },
        environment: Environment::Native,
        os: Detection::available(
            OsInfo {
                family: OsFamily::Linux,
                name: "Example Linux".into(),
                version: Some("42".into()),
                id: Some("linux".into()),
            },
            "fixture",
        ),
        host: Detection::available(
            HostInfo {
                name: "Test Machine".into(),
                kind: HostKind::DeviceModel,
            },
            "fixture",
        ),
        kernel: Detection::available("Linux 6.8".into(), "fixture"),
        uptime: Detection::available(
            UptimeInfo {
                seconds: 90061,
                scope: Scope::OperatingSystem,
            },
            "fixture",
        ),
        cpu: Detection::available(
            CpuInfo {
                name: "Test CPU".into(),
                logical_cores: 8,
            },
            "fixture",
        ),
        memory: Detection::available(
            MemoryInfo {
                total_bytes: 8 * 1_073_741_824,
                used_bytes: 2 * 1_073_741_824,
                definition: MemoryDefinition::TotalMinusAvailable,
                scope: Scope::OperatingSystem,
            },
            "fixture",
        ),
        swap: Detection::available(
            SwapInfo {
                total_bytes: 4 * 1_073_741_824,
                used_bytes: 1_073_741_824,
            },
            "fixture",
        ),
        disk: Detection::available(
            DiskInfo {
                path: "/home/tester".into(),
                total_bytes: 500 * 1_073_741_824,
                used_bytes: 100 * 1_073_741_824,
                available_bytes: 400 * 1_073_741_824,
            },
            "fixture",
        ),
        shell: Detection::available(
            ShellInfo {
                name: "zsh".into(),
                kind: ShellKind::CurrentParent,
            },
            "fixture",
        ),
        terminal: Detection::available(
            TerminalInfo {
                name: "Test Terminal".into(),
                version: Some("1.2".into()),
            },
            "fixture",
        ),
        local_ip: Detection::available(
            LocalIpInfo {
                addresses: vec![InterfaceAddress {
                    interface: Some("eth0".into()),
                    address: std::net::Ipv4Addr::new(192, 0, 2, 10),
                }],
            },
            "fixture",
        ),
    }
}

fn options(columns: usize) -> Options {
    Options {
        columns,
        color: false,
        unicode: true,
        logo: true,
    }
}

#[test]
fn fixed_plain_render_is_aligned() {
    let output = terminal::render(
        &fixture(),
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert_eq!(
        output,
        concat!(
            "tester@host\n",
            "------------\n",
            "OS      Example Linux 42\n",
            "Host    Test Machine\n",
            "Kernel  Linux 6.8\n",
            "Uptime  1d 1h 1m\n",
            "CPU     Test CPU (8 logical)\n",
            "Memory  2.00 / 8.00 GiB (25%)\n",
            "Swap    1.00 / 4.00 GiB (25%)\n",
            "Disk    100.00 / 500.00 GiB (20%) on /home/tester\n",
            "Shell   zsh\n",
            "Term    Test Terminal 1.2\n",
            "IP      192.0.2.10 (eth0)\n",
            "------------\n",
            "o o o o o o o o\n"
        )
    );
}

#[test]
fn no_swap_reads_as_none_and_every_address_is_listed() {
    let mut snapshot = fixture();
    snapshot.swap = Detection::available(
        SwapInfo {
            total_bytes: 0,
            used_bytes: 0,
        },
        "fixture",
    );
    snapshot.local_ip = Detection::available(
        LocalIpInfo {
            addresses: vec![
                InterfaceAddress {
                    interface: Some("eth0".into()),
                    address: std::net::Ipv4Addr::new(192, 0, 2, 10),
                },
                InterfaceAddress {
                    interface: Some("docker0".into()),
                    address: std::net::Ipv4Addr::new(172, 17, 0, 1),
                },
            ],
        },
        "fixture",
    );
    let output = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert!(output.contains("Swap    none"), "{output}");
    assert!(
        output.contains("IP      192.0.2.10 (eth0), 172.17.0.1 (docker0)"),
        "{output}"
    );
}

#[test]
fn an_unsupported_field_states_that_rather_than_a_number() {
    let mut snapshot = fixture();
    snapshot.swap = Detection::unsupported("This platform exposes no page-file figure");
    let output = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert!(output.contains("Swap    Unsupported\n"), "{output}");
}

#[test]
fn an_unnamed_interface_does_not_render_empty_parentheses() {
    let mut snapshot = fixture();
    snapshot.local_ip = Detection::available(
        LocalIpInfo {
            addresses: vec![InterfaceAddress {
                interface: None,
                address: std::net::Ipv4Addr::new(203, 0, 113, 7),
            }],
        },
        "fixture",
    );
    let output = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert!(output.contains("IP      203.0.113.7\n"), "{output}");
    assert!(!output.contains("()"), "{output}");
}

#[test]
fn logo_height_does_not_drop_information() {
    for art in ["x", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15"] {
        let logo = logo::Logo {
            name: "fixture",
            art,
            color: AnsiColor::Cyan,
        };
        let text = terminal::render_with_logo(&fixture(), options(80), &logo);
        assert!(text.contains("Shell   zsh"));
        assert!(text.contains("Memory  2.00 / 8.00 GiB (25%)"));
        assert!(text.contains(art.lines().last().unwrap()));
        assert!(text.lines().all(|line| layout::width(line) <= 80));
    }
}

#[test]
fn narrow_and_unicode_layouts_fit_every_column() {
    let mut snapshot = fixture();
    snapshot.cpu = Detection::available(
        CpuInfo {
            name: "超长处理器 e\u{301} 👩‍🔬 ".repeat(12),
            logical_cores: 128,
        },
        "fixture",
    );
    snapshot.identity.username = Detection::available("中文e\u{301}".into(), "fixture");
    for columns in [0, 1, 2, 8, 10, 11, 25, 40, 60, 80, 120] {
        let text = terminal::render(&snapshot, options(columns));
        assert!(
            text.lines()
                .all(|line| layout::width(line) <= columns.max(1)),
            "width {columns}: {text}"
        );
        if columns >= 25 {
            assert!(text.contains("e\u{301}"));
        }
        assert!(!text.contains('\x1b'));
    }
    let narrow = terminal::render(&snapshot, options(25));
    assert!(narrow.lines().next().unwrap().starts_with("中文"));
}

#[test]
fn every_column_count_and_capability_combination_stays_in_bounds() {
    let mut snapshot = fixture();
    snapshot.cpu = Detection::available(
        CpuInfo {
            name: "x".repeat(200),
            logical_cores: 256,
        },
        "fixture",
    );
    snapshot.identity.hostname = Detection::available("h".repeat(200), "fixture");
    snapshot.memory = Detection::error("fixture", ErrorCode::Io, "y".repeat(200));
    for columns in (0..=260).chain([1_000, 4_096, 100_000]) {
        for color in [false, true] {
            for unicode in [false, true] {
                let text = terminal::render(
                    &snapshot,
                    Options {
                        columns,
                        color,
                        unicode,
                        logo: true,
                    },
                );
                // Generated escapes occupy no display cells, so visual width is
                // measured after removing them.
                for line in text.lines() {
                    let visible = layout::sanitize(line);
                    assert!(
                        layout::width(&visible) <= columns.max(1),
                        "columns={columns} color={color} unicode={unicode}: {line:?}"
                    );
                }
                if !color {
                    assert!(
                        !text.contains('\x1b'),
                        "columns={columns} unicode={unicode}: unexpected escapes"
                    );
                } else if text.lines().any(|line| !layout::sanitize(line).is_empty()) {
                    assert!(
                        text.contains('\x1b'),
                        "columns={columns} unicode={unicode}: missing escapes"
                    );
                }
                assert!(text.ends_with('\n'), "columns={columns}: missing newline");
            }
        }
    }
}

/// Styling must be applied after layout, so a colored render is the plain render
/// with escapes inserted and nothing else changed.
#[test]
fn color_never_moves_a_character_across_columns() {
    for columns in [12, 20, 36, 53, 54, 80, 120, 200] {
        let plain = terminal::render(
            &fixture(),
            Options {
                color: false,
                ..options(columns)
            },
        );
        let colored = terminal::render(
            &fixture(),
            Options {
                color: true,
                ..options(columns)
            },
        );
        assert_eq!(
            colored.lines().map(layout::sanitize).collect::<Vec<_>>(),
            plain.lines().collect::<Vec<_>>(),
            "columns={columns}"
        );
    }
}

#[test]
fn accent_of_empty_text_produces_nothing() {
    assert_eq!(theme::accent("", AnsiColor::Cyan, true), "");
    assert_eq!(theme::accent("", AnsiColor::Cyan, false), "");
    assert!(theme::accent("x", AnsiColor::Cyan, true).contains('x'));
    assert_eq!(theme::accent("x", AnsiColor::Cyan, false), "x");
}

/// A styled span is written as `<open><text><reset>`, and `open` is itself
/// several escape sequences, so adjacency alone is normal. What only an empty
/// span can produce is a reset sitting directly where the opening ended;
/// anstyle renders that reset as `ESC [ 0 m`.
fn has_empty_styled_span(line: &str) -> bool {
    let mut after_previous = 0;
    for (start, _) in line.match_indices('\x1b') {
        let Some(parameters) = line.get(start + 2..) else {
            return false;
        };
        // A CSI sequence ends at its first byte in the final ASCII range.
        let end = start
            + 2
            + parameters
                .find(|c| ('\u{40}'..='\u{7e}').contains(&c))
                .map_or(0, |offset| offset + 1);
        if start == after_previous && line.get(start..end) == Some("\u{1b}[0m") {
            return true;
        }
        after_previous = end;
    }
    false
}

/// A logo shorter than the information column has no line for the remaining
/// rows, which must not add an empty styled span to those rows.
#[test]
fn rows_beyond_a_short_logo_carry_no_empty_styled_span() {
    let short = logo::Logo {
        name: "short",
        art: "x",
        color: AnsiColor::Cyan,
    };
    let text = terminal::render_with_logo(
        &fixture(),
        Options {
            color: true,
            ..options(80)
        },
        &short,
    );
    assert!(
        text.contains("Shell"),
        "the information column must survive"
    );
    // Guard the detector itself against silently matching nothing.
    assert!(has_empty_styled_span("\u{1b}[1m\u{1b}[36m\u{1b}[0mx"));
    assert!(!has_empty_styled_span("\u{1b}[1m\u{1b}[36mx\u{1b}[0m"));
    for line in text.lines() {
        assert!(!has_empty_styled_span(line), "empty styled span: {line:?}");
    }
}

#[test]
fn terminal_scrubs_protocols_but_json_keeps_raw_data() {
    let mut snapshot = fixture();
    let raw = "Safe\x1b[2J\x1b]52;c;c2VjcmV0\x07\u{202e}Name";
    snapshot.host = Detection::available(
        HostInfo {
            name: raw.into(),
            kind: HostKind::Hostname,
        },
        "fixture",
    );
    let text = terminal::render(&snapshot, options(80));
    assert!(text.contains("SafeName (hostname)"));
    assert!(!text.contains('\x1b'));
    assert!(!text.contains("c2VjcmV0"));
    let value: serde_json::Value = serde_json::from_str(&json::render(&snapshot).unwrap()).unwrap();
    assert_eq!(value["host"]["value"]["name"], raw);
}

#[test]
fn all_detection_states_remain_distinct() {
    let mut snapshot = fixture();
    snapshot.host = Detection::unavailable("No model");
    snapshot.kernel = Detection::Unsupported {
        reason: "Fixture unsupported".into(),
    };
    snapshot.shell = Detection::error("fixture shell", ErrorCode::ApiFailed, "Permission denied");
    snapshot.cpu = Detection::estimated(
        CpuInfo {
            name: "Approximate".into(),
            logical_cores: 4,
        },
        "fixture",
        "Estimate",
    );
    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            ..options(120)
        },
    );
    assert!(text.contains("Host    Unavailable"));
    assert!(text.contains("Kernel  Unsupported"));
    assert!(text.contains("CPU     ~ Approximate"));
    assert!(text.contains("Shell   Error (fixture shell: Permission denied)"));
    assert!(text.contains("Memory  2.00 / 8.00 GiB (25%)"));
    let value: serde_json::Value = serde_json::from_str(&json::render(&snapshot).unwrap()).unwrap();
    assert_eq!(value["host"]["status"], "unavailable");
    assert!(value["host"].get("value").is_none());
    assert_eq!(value["kernel"]["status"], "unsupported");
    assert_eq!(value["shell"]["status"], "error");
    assert_eq!(value["shell"]["code"], "api_failed");
    assert_eq!(value["cpu"]["confidence"], "estimated");
    assert_eq!(value["uptime"]["value"]["seconds"], 90061);
    assert_eq!(
        value["memory"]["value"]["total_bytes"],
        8 * 1_073_741_824_u64
    );
}

#[test]
fn mapping_does_not_erase_errors_or_estimation() {
    let error: Detection<u64> = Detection::error("proc", ErrorCode::Io, "Denied");
    assert_eq!(
        error.map(|v| v.to_string()),
        Detection::<String>::error("proc", ErrorCode::Io, "Denied")
    );
    let estimate = Detection::estimated(1, "source", "note").map(|v| v + 1);
    assert!(matches!(
        estimate,
        Detection::Available {
            value: 2,
            confidence: Confidence::Estimated,
            ..
        }
    ));
}

#[test]
fn logo_mapping_and_builtin_assets() {
    for id in [
        "macos", "windows", "linux", "arch", "debian", "ubuntu", "fedora", "gentoo", "opensuse",
        "nixos",
    ] {
        let logo = logo::select(Some(id), None);
        assert_eq!(logo.name, id);
        assert!(logo.art.is_ascii());
        assert!(logo.art.lines().all(|line| layout::width(line) <= 24));
    }
    assert_eq!(
        logo::select(Some("opensuse-tumbleweed"), None).name,
        "opensuse"
    );
    assert_eq!(logo::select(None, None).name, "fallback");
    assert_eq!(logo::select(Some("unknown"), None).name, "fallback");
    // The family is what keeps an unrecognized Linux host off the fallback.
    assert_eq!(
        logo::select(Some("some-unknown-distro"), Some(&OsFamily::Linux)).name,
        "linux"
    );
    assert_eq!(logo::select(None, Some(&OsFamily::Linux)).name, "linux");
    assert_eq!(
        logo::select(Some("unknown"), Some(&OsFamily::Windows)).name,
        "fallback"
    );
}

/// A Linux distribution without its own artwork must render the generic Linux
/// logo, not the unidentified-system emblem.
#[test]
fn unknown_linux_distribution_renders_the_generic_linux_logo() {
    let mut snapshot = fixture();
    snapshot.os = Detection::available(
        OsInfo {
            family: OsFamily::Linux,
            name: "Example Linux".into(),
            version: None,
            id: Some("some-unknown-distro".into()),
        },
        "fixture",
    );
    let text = terminal::render(&snapshot, options(80));
    let generic = logo::select(Some("linux"), None).art;
    let fallback = logo::select(None, None).art;
    for line in generic.lines().filter(|line| !line.trim().is_empty()) {
        assert!(text.contains(line), "missing generic linux line {line:?}");
    }
    for line in fallback.lines().filter(|line| !line.trim().is_empty()) {
        assert!(!text.contains(line), "fallback leaked into {line:?}");
    }
}

#[test]
fn color_only_adds_generated_ansi_after_layout() {
    let plain = terminal::render(&fixture(), options(80));
    let colored = terminal::render(
        &fixture(),
        Options {
            color: true,
            ..options(80)
        },
    );
    assert!(colored.contains('\x1b'));
    // Sanitize each line independently, since sanitization also flattens newlines.
    assert_eq!(
        colored.lines().map(layout::sanitize).collect::<Vec<_>>(),
        plain.lines().collect::<Vec<_>>()
    );
}

#[test]
fn shell_recognition_does_not_mislabel_a_runner() {
    assert_eq!(recognized_name("/bin/zsh"), Some("zsh".into()));
    assert_eq!(
        recognized_name(r"C:\Program Files\PowerShell\7\pwsh.exe"),
        Some("pwsh".into())
    );
    assert_eq!(
        recognized_name(r"C:\Windows\System32\cmd.EXE"),
        Some("cmd".into())
    );
    assert_eq!(recognized_name("/usr/bin/python3"), None);
    assert_eq!(recognized_name("/usr/bin/notbash"), None);
}
