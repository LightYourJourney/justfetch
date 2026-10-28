use justfetch::{
    detect::shell::recognized_name,
    model::*,
    module::ModuleId,
    privacy,
    render::{
        colour::{Colour, Depth},
        json,
        layout::{self, Layout},
        logo,
        terminal::{self, Options},
        theme::{self, Theme},
    },
};

fn default_theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(|| theme::built_in("default").expect("built in"))
}

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
                kind: TerminalKind::Application,
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

fn options(columns: usize) -> Options<'static> {
    Options {
        columns,
        depth: Depth::None,
        unicode: true,
        logo: true,
        logo_explicit: false,
        layout: Layout::Classic,
        modules: &ModuleId::ALL,
        labels: &[],
        separator: "  ",
        theme: default_theme(),
        accent: None,
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

    // The interface is omitted from JSON rather than sent as an empty string,
    // and the address stays a plain string rather than an Ipv4Addr debug form.
    let value: serde_json::Value = serde_json::from_str(&json::render(&snapshot).unwrap()).unwrap();
    let unnamed = &value["local_ip"]["value"]["addresses"][0];
    assert_eq!(unnamed["address"], "203.0.113.7");
    assert!(unnamed.get("interface").is_none(), "{unnamed}");

    let named: serde_json::Value =
        serde_json::from_str(&json::render(&fixture()).unwrap()).unwrap();
    assert_eq!(
        named["local_ip"]["value"]["addresses"][0]["interface"],
        "eth0"
    );
}

#[test]
fn privacy_withholds_identity_but_keeps_the_counters() {
    let mut snapshot = fixture();
    privacy::redact(&mut snapshot);
    assert!(matches!(snapshot.identity.username, Detection::Redacted));
    assert!(matches!(snapshot.identity.hostname, Detection::Redacted));
    assert!(matches!(snapshot.local_ip, Detection::Redacted));
    // The fixture's Host is a device model, not the hostname, so it survives.
    assert!(snapshot.host.value().is_some());
    // The path is shortened, not withheld: the field stays available because
    // its counters are exact. That distinction is documented, so it is pinned.
    assert!(matches!(snapshot.disk, Detection::Available { .. }));
    let disk = snapshot.disk.value().unwrap();
    assert_eq!(disk.path, "~");
    assert_eq!(disk.total_bytes, 500 * 1_073_741_824);

    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert_eq!(text.lines().next(), Some("redacted@redacted"), "{text}");
    assert!(
        text.contains("Disk    100.00 / 500.00 GiB (20%) on ~"),
        "{text}"
    );
    assert!(!text.contains("tester"), "{text}");
    assert!(!text.contains("192.0.2.10"), "{text}");
}

#[test]
fn privacy_also_withholds_a_host_row_that_is_the_hostname() {
    let mut snapshot = fixture();
    snapshot.host = Detection::available(
        HostInfo {
            name: "host".into(),
            kind: HostKind::Hostname,
        },
        "fixture",
    );
    privacy::redact(&mut snapshot);
    assert!(matches!(snapshot.host, Detection::Redacted));

    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            ..options(80)
        },
    );
    assert!(!text.contains("host (hostname)"), "{text}");
    assert!(text.contains("Host    redacted"), "{text}");
}

#[test]
fn a_narrow_colored_row_still_accents_its_label() {
    fn visible(line: &str) -> String {
        let mut out = String::new();
        let mut escape = false;
        for ch in line.chars() {
            if escape {
                if ch.is_ascii_alphabetic() {
                    escape = false;
                }
            } else if ch == '\u{1b}' {
                escape = true;
            } else {
                out.push(ch);
            }
        }
        out
    }
    // Styling is applied by cell range, so a line that begins inside the label
    // column still carries the label's colour — including at widths narrower
    // than the column itself. From two columns upward the label fits on one
    // line, which is what lets the test find the row.
    for columns in 2..=12 {
        let text = terminal::render(
            &fixture(),
            Options {
                depth: Depth::Ansi16,
                logo: false,
                ..options(columns)
            },
        );
        let row = text
            .lines()
            .find(|line| visible(line).starts_with("OS"))
            .unwrap_or_else(|| panic!("no OS row at {columns} columns: {text:?}"));
        assert!(
            row.starts_with('\u{1b}'),
            "the label lost its accent at {columns} columns: {row:?}"
        );
    }
}

#[test]
fn logo_height_does_not_drop_information() {
    for art in ["x", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15"] {
        let logo = logo::Logo {
            name: "fixture".to_owned(),
            art: art.to_owned(),
            color: Some(Colour::Ansi(anstyle::AnsiColor::Cyan)),
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
            // Wide characters and an emoji sequence, not only ASCII: display
            // width, not byte length, is what the layout must respect.
            name: format!("处理器🖥️{}", "x".repeat(200)),
            logical_cores: 256,
        },
        "fixture",
    );
    snapshot.identity.hostname = Detection::available("中".repeat(100), "fixture");
    snapshot.memory = Detection::error("fixture", ErrorCode::Io, "y".repeat(200));
    for columns in (0..=260).chain([1_000, 4_096, 100_000]) {
        for layout in Layout::ALL {
            for (color, depth) in [(false, Depth::None), (true, Depth::Ansi16)] {
                for unicode in [false, true] {
                    let text = terminal::render(
                        &snapshot,
                        Options {
                            depth,
                            unicode,
                            layout,
                            logo: true,
                            ..options(columns)
                        },
                    );
                    // Generated escapes occupy no display cells, so visual width is
                    // measured after removing them.
                    for line in text.lines() {
                        let visible = layout::sanitize(line);
                        assert!(
                            layout::width(&visible) <= columns.max(1),
                            "columns={columns} layout={layout} color={color} unicode={unicode}: {line:?}"
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
}

/// Styling must be applied after layout, so a colored render is the plain render
/// with escapes inserted and nothing else changed.
#[test]
fn color_never_moves_a_character_across_columns() {
    for columns in [12, 20, 36, 53, 54, 80, 120, 200] {
        for layout in Layout::ALL {
            let plain = terminal::render(
                &fixture(),
                Options {
                    depth: Depth::None,
                    layout,
                    ..options(columns)
                },
            );
            let coloured = terminal::render(
                &fixture(),
                Options {
                    depth: Depth::Ansi16,
                    layout,
                    ..options(columns)
                },
            );
            assert_eq!(
                coloured.lines().map(layout::sanitize).collect::<Vec<_>>(),
                plain.lines().collect::<Vec<_>>(),
                "columns={columns} layout={layout}"
            );
        }
    }
}

#[test]
fn style_of_empty_text_produces_nothing() {
    use anstyle::Style;
    let coloured = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)))
        .bold();
    assert_eq!(theme::styled("", coloured), "");
    assert_eq!(theme::styled("", Style::new()), "");
    assert!(theme::styled("x", coloured).contains('x'));
    assert_eq!(theme::styled("x", Style::new()), "x");
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
        name: "short".to_owned(),
        art: "x".to_owned(),
        color: Some(Colour::Ansi(anstyle::AnsiColor::Cyan)),
    };
    let text = terminal::render_with_logo(
        &fixture(),
        Options {
            depth: Depth::Ansi16,
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

/// Every distribution id that has artwork of its own.
const BUILT_IN_LOGO_IDS: [&str; 10] = [
    "macos", "windows", "linux", "arch", "debian", "ubuntu", "fedora", "gentoo", "opensuse",
    "nixos",
];

#[test]
fn logo_mapping_and_builtin_assets() {
    for id in BUILT_IN_LOGO_IDS {
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

#[test]
fn each_built_in_logo_is_its_own_artwork() {
    // Asserting a logo's name, ASCII-ness and width would still pass if two
    // distributions were shipped the same picture; comparing the art does not.
    // Naming the expected file per id is what catches two pictures being
    // swapped; comparing each logo only against the others would not.
    let expected = [
        ("macos", include_str!("../assets/logos/macos.txt")),
        ("windows", include_str!("../assets/logos/windows.txt")),
        ("linux", include_str!("../assets/logos/linux.txt")),
        ("arch", include_str!("../assets/logos/arch.txt")),
        ("debian", include_str!("../assets/logos/debian.txt")),
        ("ubuntu", include_str!("../assets/logos/ubuntu.txt")),
        ("fedora", include_str!("../assets/logos/fedora.txt")),
        ("gentoo", include_str!("../assets/logos/gentoo.txt")),
        ("opensuse", include_str!("../assets/logos/opensuse.txt")),
        ("nixos", include_str!("../assets/logos/nixos.txt")),
    ];
    assert_eq!(expected.len(), BUILT_IN_LOGO_IDS.len());
    for (id, art) in expected {
        let selected = logo::select(Some(id), None);
        assert_eq!(selected.art, art, "{id} is not showing its own artwork");
    }
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
            depth: Depth::Ansi16,
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

#[test]
fn the_selection_decides_which_rows_exist_and_in_what_order() {
    let snapshot = fixture();
    let modules = [ModuleId::Memory, ModuleId::Os];
    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            modules: &modules,
            ..options(80)
        },
    );
    let memory = text.find("Memory").expect("Memory row");
    let os = text.find("OS ").expect("OS row");
    assert!(memory < os, "rows follow the selection order:\n{text}");
    for absent in ["Kernel", "Uptime", "Shell", "Disk"] {
        assert!(!text.contains(absent), "{absent} was not selected:\n{text}");
    }
}

#[test]
fn custom_labels_and_separator_are_used_and_sanitised() {
    let snapshot = fixture();
    let labels = [
        (ModuleId::Cpu, "Processor".to_owned()),
        // A label is configuration input, so an escape in it must not reach the
        // terminal.
        (ModuleId::Os, "Sys\u{1b}[31m".to_owned()),
    ];
    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            labels: &labels,
            separator: " :: ",
            ..options(80)
        },
    );
    assert!(text.contains("Processor :: Test CPU (8 logical)"), "{text}");
    assert!(
        text.lines()
            .any(|line| line.starts_with("Sys") && line.contains(":: Example Linux 42")),
        "{text}"
    );
    assert!(!text.contains('\u{1b}'), "an escape survived:\n{text:?}");
}

#[test]
fn a_longer_label_widens_the_column_instead_of_breaking_alignment() {
    let snapshot = fixture();
    let labels = [(ModuleId::Os, "Operating system".to_owned())];
    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            labels: &labels,
            ..options(80)
        },
    );
    let short = text
        .lines()
        .find(|line| line.starts_with("CPU"))
        .expect("a CPU row");
    assert!(
        short.starts_with("CPU             "),
        "short labels are padded to the new column width: {short:?}"
    );
}

#[test]
fn a_subset_keeps_the_default_label_column_width() {
    // The column must not shrink to fit a subset: the same row should align the
    // same way whatever else is on screen.
    let snapshot = fixture();
    let modules = [ModuleId::Os, ModuleId::Cpu];
    let text = terminal::render(
        &snapshot,
        Options {
            logo: false,
            unicode: false,
            modules: &modules,
            ..options(80)
        },
    );
    let os = text.lines().find(|line| line.starts_with("OS")).unwrap();
    assert!(
        os.starts_with("OS      "),
        "the label column kept its default width: {os:?}"
    );
}

#[test]
fn a_configured_accent_colours_the_text() {
    let snapshot = fixture();
    let plain = terminal::render(
        &snapshot,
        Options {
            depth: Depth::Ansi16,
            logo: false,
            accent: None,
            ..options(80)
        },
    );
    let cyan = terminal::render(
        &snapshot,
        Options {
            depth: Depth::Ansi16,
            logo: false,
            accent: Some(Colour::Ansi(anstyle::AnsiColor::Cyan)),
            ..options(80)
        },
    );
    assert_ne!(plain, cyan, "the accent should change the emitted colour");
    assert!(cyan.contains('\u{1b}'));
}

#[test]
fn every_built_in_theme_changes_only_the_colour() {
    let plain = terminal::render(&fixture(), options(80));
    for name in theme::names() {
        let built_in = theme::built_in(name).expect("a listed theme exists");
        let coloured = terminal::render(
            &fixture(),
            Options {
                depth: Depth::TrueColor,
                theme: &built_in,
                ..options(80)
            },
        );
        assert!(coloured.contains('\u{1b}'), "{name} emitted no styling");
        // The swatch row is the theme's own palette, so its length is the one
        // thing a theme is allowed to change; everything else must be identical.
        // A swatch row is made only of dots and spaces; matching on the
        // character alone would also drop rows whose text contains an `o`.
        let information = |text: &str| {
            text.lines()
                .map(layout::sanitize)
                .filter(|line| !line.chars().all(|c| c == '●' || c == 'o' || c == ' '))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            information(&coloured),
            information(&plain),
            "{name} moved or changed the text"
        );
    }
}

#[test]
fn each_layout_shows_the_same_information() {
    for layout in Layout::ALL {
        let output = terminal::render(
            &fixture(),
            Options {
                layout,
                logo: false,
                ..options(80)
            },
        );
        for module in ModuleId::ALL {
            assert!(output.contains(module.label()), "{layout}: {module:?}");
        }
        assert!(output.contains("192.0.2.10"), "{layout} lost a value");
        assert!(output.contains("Test Machine"), "{layout} lost a value");
    }
}

#[test]
fn the_layouts_differ_in_the_way_they_describe() {
    let render = |layout| {
        terminal::render(
            &fixture(),
            Options {
                layout,
                ..options(80)
            },
        )
    };
    let classic = render(Layout::Classic);
    assert!(classic.contains('─'), "classic has no rule");
    assert!(classic.contains('●'), "classic has no swatches");
    assert!(
        classic.lines().any(|line| line.contains("o   o")),
        "classic has no artwork"
    );

    let compact = render(Layout::Compact);
    assert!(!compact.contains('─') && !compact.contains('●'));
    assert!(compact.contains("OS      Example Linux 42"), "{compact}");

    let minimal = render(Layout::Minimal);
    assert!(minimal.contains("OS: Example Linux 42"), "{minimal}");
    assert!(!minimal.contains('─'));

    let vertical = render(Layout::Vertical);
    let first = vertical.lines().next().unwrap_or_default();
    assert!(
        first.trim().is_empty() || first.starts_with(' '),
        "the artwork is not centred"
    );
    assert!(vertical.contains('─') && vertical.contains('●'));
    assert!(vertical.find("o   o").unwrap() < vertical.find("tester@host").unwrap());
}

#[test]
fn decor_disappears_before_information_when_the_terminal_is_narrow() {
    for layout in [Layout::Classic, Layout::Vertical] {
        let output = terminal::render(
            &fixture(),
            Options {
                layout,
                logo: false,
                ..options(20)
            },
        );
        assert!(!output.contains('─'), "{layout} kept a rule at 20 columns");
        assert!(
            !output.contains('●'),
            "{layout} kept swatches at 20 columns"
        );
        assert!(output.contains("Kernel"), "{layout} lost a row");
    }
}

#[test]
fn a_custom_logo_is_drawn_and_never_emits_its_own_escapes() {
    let custom = logo::parse_custom("  /\\\n /  \\\x1b[31m\n", "mine", "logo.txt").unwrap();
    let output = terminal::render_with_logo(&fixture(), options(80), &custom);
    assert!(output.contains("  /\\"), "{output}");
    assert!(output.contains(" /  \\"), "{output}");
    assert!(
        !output.contains('\u{1b}'),
        "the file's escape reached the terminal"
    );
}

#[test]
fn colour_depth_is_visible_in_what_is_emitted() {
    let nord = theme::built_in("nord").unwrap();
    let at = |depth| {
        terminal::render(
            &fixture(),
            Options {
                depth,
                theme: &nord,
                logo: false,
                ..options(80)
            },
        )
    };
    assert!(at(Depth::TrueColor).contains("\u{1b}[38;2;"));
    assert!(at(Depth::Ansi256).contains("\u{1b}[38;5;"));
    let sixteen = at(Depth::Ansi16);
    assert!(!sixteen.contains("38;2;") && !sixteen.contains("38;5;"));
    assert!(sixteen.contains('\u{1b}'), "16 colours is still colour");
    assert!(!at(Depth::None).contains('\u{1b}'));
}

#[test]
fn a_custom_theme_that_leaves_a_role_unset_falls_back() {
    let custom = theme::parse("name = \"mine\"\nlogo = \"cyan\"\n", "theme.toml").unwrap();
    let output = terminal::render(
        &fixture(),
        Options {
            depth: Depth::Ansi16,
            theme: &custom,
            logo: false,
            ..options(80)
        },
    );
    // Title and labels take the logo colour; values stay unstyled.
    let title = output.lines().next().unwrap();
    assert!(title.contains("36"), "{title:?}");
}

#[test]
fn a_wide_or_emoji_label_keeps_every_value_in_one_column() {
    for label in ["处理器类型", "🖥️ CPU", "e\u{301}clair"] {
        let labels = vec![(ModuleId::Cpu, label.to_owned())];
        let output = terminal::render(
            &fixture(),
            Options {
                labels: &labels,
                logo: false,
                ..options(80)
            },
        );
        let expected = layout::width(label).max(6) + 2;
        for value in ["Example Linux 42", "Linux 6.8", "2.00 / 8.00", "zsh"] {
            let line = output
                .lines()
                .find(|line| line.contains(value))
                .unwrap_or_else(|| panic!("{label}: no row for {value}\n{output}"));
            let start = line.find(value).unwrap();
            assert_eq!(
                layout::width(&line[..start]),
                expected,
                "{label}: `{value}` starts at the wrong column"
            );
        }
    }
}

#[test]
fn an_explicitly_chosen_logo_is_stacked_rather_than_dropped() {
    let art = logo::built_in("arch").expect("built in");
    let narrow = |explicit| {
        terminal::render_with_logo(
            &fixture(),
            Options {
                logo_explicit: explicit,
                ..options(40)
            },
            &art,
        )
    };
    // The automatic choice still gives way to the information.
    assert!(!narrow(false).contains("/\\"));
    // An explicit one moves above it instead of vanishing.
    let stacked = narrow(true);
    assert!(stacked.contains("/\\"), "{stacked}");
    assert!(stacked.contains("Kernel"), "the information must survive");
    for line in stacked.lines() {
        assert!(layout::width(line) <= 40, "{line:?}");
    }
    // Wide enough to sit beside the information, the arrangement is unchanged.
    let beside = terminal::render_with_logo(
        &fixture(),
        Options {
            logo_explicit: true,
            ..options(80)
        },
        &art,
    );
    assert!(
        beside.lines().any(|line| line.starts_with("         /\\")),
        "{beside}"
    );
}

#[test]
fn artwork_exactly_as_wide_as_the_terminal_is_drawn_when_chosen() {
    let art = logo::parse_custom("#".repeat(10).as_str(), "mine", "art.txt").unwrap();
    let at = |columns| {
        terminal::render_with_logo(
            &fixture(),
            Options {
                logo_explicit: true,
                ..options(columns)
            },
            &art,
        )
    };
    // Ten columns of artwork in a ten-column terminal fits; it must be drawn.
    assert!(at(10).contains("##########"), "{}", at(10));
    // One column short cannot be drawn at all.
    assert!(!at(9).contains("##########"));
}

#[test]
fn a_row_too_narrow_for_the_column_still_styles_what_it_can() {
    let nord = theme::built_in("nord").expect("built in");
    let output = terminal::render(
        &fixture(),
        Options {
            depth: Depth::TrueColor,
            logo: false,
            theme: &nord,
            ..options(10)
        },
    );
    let row = output
        .lines()
        .find(|line| layout::sanitize(line).starts_with("OS "))
        .unwrap_or_else(|| panic!("no OS row:\n{output}"));
    // Label styled, separator plain, then a styled value fragment.
    let label_end = row.find("\u{1b}[0m").expect("the label is styled");
    assert!(row[..label_end].contains("OS"));
    assert!(row[label_end..].starts_with("\u{1b}[0m  "), "{row:?}");
    assert!(
        row[label_end + 3..].contains('\u{1b}'),
        "value lost its colour"
    );
}

#[test]
fn a_separator_split_across_lines_does_not_leave_the_value_plain() {
    let nord = theme::built_in("nord").expect("built in");
    // Seven columns holds the six-cell label column and one cell of separator,
    // so the separator straddles two lines.
    let output = terminal::render(
        &fixture(),
        Options {
            depth: Depth::TrueColor,
            logo: false,
            theme: &nord,
            ..options(7)
        },
    );
    let lines: Vec<&str> = output.lines().collect();
    let row = lines
        .iter()
        .position(|line| layout::sanitize(line).starts_with("OS"))
        .unwrap_or_else(|| panic!("no OS row:\n{output}"));
    // The line may open with a plain separator fragment; what must not be
    // plain is the value that follows it.
    let continuation = lines[row + 1].trim_start();
    assert!(
        continuation.starts_with("\u{1b}[38;2;"),
        "the continuation lost its colour: {continuation:?}"
    );
}
