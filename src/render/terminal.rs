use super::{
    layout::{sanitize, width, wrap},
    logo, theme,
};
use crate::model::*;
use crate::module::ModuleId;
use anstyle::AnsiColor;
use std::ffi::OsStr;

#[derive(Debug, Clone, Copy)]
pub struct Options<'a> {
    pub columns: usize,
    pub color: bool,
    pub unicode: bool,
    pub logo: bool,
    /// Which rows to draw, in this order.
    pub modules: &'a [ModuleId],
    /// Custom labels, by module. Anything absent keeps its default label.
    pub labels: &'a [(ModuleId, String)],
    /// Text between the label column and the value.
    pub separator: &'a str,
    /// Accent for the title and labels. `None` uses the logo's own colour.
    pub accent: Option<AnsiColor>,
}

/// Pure policy: NO_COLOR has effect only when nonempty, per no-color.org.
pub fn color_allowed(
    tty: bool,
    no_color: bool,
    env_no_color: Option<&OsStr>,
    term: Option<&str>,
) -> bool {
    tty && !no_color && env_no_color.is_none_or(OsStr::is_empty) && term != Some("dumb")
}

const GIBIBYTE: f64 = 1_073_741_824.0;

/// The `used / total GiB (n%)` shape that memory, swap and disk all share.
fn capacity(used: u64, total: u64) -> String {
    format!(
        "{:.2} / {:.2} GiB ({:.0}%)",
        used as f64 / GIBIBYTE,
        total as f64 / GIBIBYTE,
        used as f64 / total.max(1) as f64 * 100.0
    )
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{seconds}s")
    }
}

fn display<T>(detection: &Detection<T>, f: impl FnOnce(&T) -> String) -> String {
    match detection {
        Detection::Available {
            value, confidence, ..
        } => {
            let text = f(value);
            if *confidence == Confidence::Estimated {
                format!("~ {text}")
            } else {
                text
            }
        }
        Detection::Unavailable { .. } => "Unavailable".into(),
        Detection::Unsupported { .. } => "Unsupported".into(),
        Detection::Redacted => "redacted".into(),
        // Not reachable through the renderer, which skips deselected modules,
        // but the state must still have a textual form.
        Detection::Deselected => "not selected".into(),
        Detection::Error {
            source, message, ..
        } => format!("Error ({source}: {message})"),
    }
}

fn plain(text: &str, unicode: bool) -> String {
    let text = sanitize(text);
    if unicode {
        text
    } else {
        text.chars()
            .map(|c| if c.is_ascii() { c } else { '?' })
            .collect()
    }
}

pub fn render(snapshot: &SystemSnapshot, options: Options<'_>) -> String {
    let os = snapshot.os.value();
    let logo = logo::select(os.and_then(|os| os.id.as_deref()), os.map(|os| &os.family));
    render_with_logo(snapshot, options, &logo)
}

/// Explicit logo input also exercises shorter and taller custom fixtures.
pub fn render_with_logo(
    snapshot: &SystemSnapshot,
    options: Options<'_>,
    logo: &logo::Logo,
) -> String {
    let columns = options.columns.clamp(1, 4096);
    let logo_lines: Vec<&str> = logo.art.lines().collect();
    let logo_width = logo_lines.iter().map(|l| width(l)).max().unwrap_or(0);
    let show_logo = options.logo && columns >= logo_width + 3 + 36;
    let info_columns = if show_logo {
        columns - logo_width - 3
    } else {
        columns
    };
    // A configured accent colours the text; the artwork keeps its own colour.
    let accent = options.accent.unwrap_or(logo.color);
    let username = display(&snapshot.identity.username, Clone::clone);
    let hostname = display(&snapshot.identity.hostname, Clone::clone);
    let title = plain(&format!("{username}@{hostname}"), options.unicode);
    let divider = if options.unicode { "─" } else { "-" }
        .repeat(width(&title).clamp(12, 40).min(info_columns));
    let mut lines: Vec<Line> = wrap(&title, info_columns)
        .into_iter()
        .enumerate()
        .map(|(index, text)| Line {
            text,
            accent: (index == 0).then_some(Accent::Whole),
        })
        .collect();
    lines.push(Line {
        text: divider.clone(),
        accent: None,
    });

    let rows = field_rows(snapshot, options.modules, options.labels);
    // Custom labels and separators are user input, so they are sanitised and
    // measured exactly like values: no escape reaches the terminal unstripped,
    // and a wide character cannot shift the column.
    let labels: Vec<String> = rows
        .iter()
        .map(|row| plain(&row.label, options.unicode))
        .collect();
    let separator = plain(options.separator, options.unicode);
    // The column is as wide as the built-in labels need, or as wide as a custom
    // label on a visible row makes it. It deliberately does not shrink to fit a
    // subset: the same row should align the same way whatever else is shown.
    let mut label_width = ModuleId::ALL
        .iter()
        .map(|module| width(module.label()))
        .max()
        .unwrap_or(0);
    for label in &labels {
        label_width = label_width.max(width(label));
    }
    let prefix_width = label_width + width(&separator);
    let continuation = " ".repeat(prefix_width);
    for (row, label) in rows.iter().zip(&labels) {
        let value = plain(&row.value, options.unicode);
        // Padded by display width, not character count, so a wide-character
        // label would still line up.
        let prefix = format!(
            "{label}{}{separator}",
            " ".repeat(label_width - width(label))
        );
        // Keep a readable continuation indent whenever enough width exists.
        if info_columns > prefix_width + 2 {
            let chunks = wrap(&value, info_columns - prefix_width);
            for (index, chunk) in chunks.into_iter().enumerate() {
                lines.push(if index == 0 {
                    Line {
                        text: format!("{prefix}{chunk}"),
                        accent: Some(Accent::Prefix(prefix.len())),
                    }
                } else {
                    Line {
                        text: format!("{continuation}{chunk}"),
                        accent: None,
                    }
                });
            }
        } else {
            for (index, text) in wrap(&format!("{prefix}{value}"), info_columns)
                .into_iter()
                .enumerate()
            {
                // A row that starts with the whole label column still gets it
                // accented; one too narrow to hold the column starts mid-label
                // and stays plain, as before.
                let accent = (index == 0
                    && text.len() >= prefix.len()
                    && text.is_char_boundary(prefix.len()))
                .then_some(Accent::Prefix(prefix.len()));
                lines.push(Line { text, accent });
            }
        }
    }
    lines.push(Line {
        text: divider,
        accent: None,
    });
    let dot = if options.unicode { "●" } else { "o" };
    let palette = [
        AnsiColor::Black,
        AnsiColor::Red,
        AnsiColor::Green,
        AnsiColor::Yellow,
        AnsiColor::Blue,
        AnsiColor::Magenta,
        AnsiColor::Cyan,
        AnsiColor::White,
    ];
    let swatches = palette
        .into_iter()
        .take(info_columns.div_ceil(2).min(8))
        .map(|color| theme::accent(dot, color, options.color))
        .collect::<Vec<_>>()
        .join(" ");
    // ANSI is added only after all plain widths and wrapping have been computed.
    let mut output = String::new();
    let row_count = lines
        .len()
        .max(if show_logo { logo_lines.len() } else { 0 });
    for index in 0..row_count {
        if show_logo {
            let line = logo_lines.get(index).copied().unwrap_or("");
            output.push_str(&theme::accent(line, logo.color, options.color));
            output.push_str(&" ".repeat(logo_width - width(line) + 3));
        }
        if let Some(line) = lines.get(index) {
            match line.accent {
                Some(Accent::Whole) => {
                    output.push_str(&theme::accent(&line.text, accent, options.color));
                }
                Some(Accent::Prefix(length)) => {
                    output.push_str(&theme::accent(&line.text[..length], accent, options.color));
                    output.push_str(&line.text[length..]);
                }
                None => output.push_str(&line.text),
            }
        }
        // Avoid trailing padding when the logo extends below the information.
        while output.ends_with(' ') {
            output.pop();
        }
        output.push('\n');
    }
    if show_logo {
        output.push_str(&" ".repeat(logo_width + 3));
    }
    output.push_str(&swatches);
    output.push('\n');
    output
}

/// One information row before layout: the label to show and the composed value.
struct Row {
    label: String,
    value: String,
}

/// A composed line plus which part of it is accented. The styling pass reads
/// this instead of slicing rows and matching label strings.
struct Line {
    text: String,
    accent: Option<Accent>,
}

enum Accent {
    /// The whole line, used for the title.
    Whole,
    /// The leading label column, measured in bytes of this line.
    Prefix(usize),
}

/// The rows to draw, in the order the user asked for. A module that is not in
/// the selection produces no row at all.
fn field_rows(
    snapshot: &SystemSnapshot,
    modules: &[ModuleId],
    labels: &[(ModuleId, String)],
) -> Vec<Row> {
    modules
        .iter()
        .map(|&module| Row {
            label: labels
                .iter()
                .find(|(id, _)| *id == module)
                .map(|(_, label)| label.clone())
                .unwrap_or_else(|| module.label().to_owned()),
            value: value_for(snapshot, module),
        })
        .collect()
}

/// The composed value for one module. One arm per module: adding a module is a
/// compile error here rather than a row that silently renders nothing.
fn value_for(snapshot: &SystemSnapshot, module: ModuleId) -> String {
    match module {
        ModuleId::Os => {
            let name = display(&snapshot.os, |os| match &os.version {
                Some(version) if !os.name.contains(version) => format!("{} {version}", os.name),
                _ => os.name.clone(),
            });
            let environment = match snapshot.environment {
                Environment::Native => "",
                Environment::Wsl { version: Some(2) } => " (WSL2)",
                Environment::Wsl { .. } => " (WSL)",
            };
            format!("{name}{environment}")
        }
        ModuleId::Host => display(&snapshot.host, |host| match host.kind {
            HostKind::DeviceModel => host.name.clone(),
            HostKind::Hostname => format!("{} (hostname)", host.name),
        }),
        ModuleId::Kernel => display(&snapshot.kernel, Clone::clone),
        ModuleId::Uptime => display(&snapshot.uptime, |value| format_uptime(value.seconds)),
        ModuleId::Cpu => display(&snapshot.cpu, |cpu| {
            format!("{} ({} logical)", cpu.name, cpu.logical_cores)
        }),
        ModuleId::Memory => display(&snapshot.memory, |mem| {
            capacity(mem.used_bytes, mem.total_bytes)
        }),
        ModuleId::Swap => display(&snapshot.swap, |swap| {
            if swap.total_bytes == 0 {
                "none".to_owned()
            } else {
                capacity(swap.used_bytes, swap.total_bytes)
            }
        }),
        ModuleId::Disk => display(&snapshot.disk, |disk| {
            format!(
                "{} on {}",
                capacity(disk.used_bytes, disk.total_bytes),
                disk.path
            )
        }),
        ModuleId::Shell => display(&snapshot.shell, |shell| match shell.kind {
            ShellKind::CurrentParent => shell.name.clone(),
            ShellKind::ConfiguredDefault => format!("{} (default)", shell.name),
        }),
        ModuleId::Terminal => display(&snapshot.terminal, |terminal| match terminal.kind {
            TerminalKind::Application => match &terminal.version {
                Some(version) => format!("{} {version}", terminal.name),
                None => terminal.name.clone(),
            },
            TerminalKind::CapabilityType => format!("{} (TERM type)", terminal.name),
        }),
        ModuleId::LocalIp => display(&snapshot.local_ip, |local_ip| {
            local_ip
                .addresses
                .iter()
                .map(|entry| match &entry.interface {
                    Some(interface) => format!("{} ({interface})", entry.address),
                    None => entry.address.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_policy_respects_environment() {
        assert!(color_allowed(true, false, None, Some("xterm")));
        assert!(!color_allowed(false, false, None, None));
        assert!(!color_allowed(true, true, None, None));
        assert!(!color_allowed(true, false, Some(OsStr::new("1")), None));
        assert!(color_allowed(true, false, Some(OsStr::new("")), None));
        assert!(!color_allowed(true, false, None, Some("dumb")));
    }
    #[cfg(unix)]
    #[test]
    fn no_color_does_not_require_utf8() {
        use std::os::unix::ffi::OsStrExt;
        assert!(!color_allowed(
            true,
            false,
            Some(OsStr::from_bytes(&[0xff])),
            Some("xterm")
        ));
    }
    /// The label column is derived from the labels themselves, so the width the
    /// styling pass slices and the width the rows are padded to are the same
    /// value by construction. `fixed_plain_render_is_aligned` pins the result.
    #[test]
    fn uptime_units() {
        assert_eq!(format_uptime(59), "59s");
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(3660), "1h 1m");
        assert_eq!(format_uptime(90061), "1d 1h 1m");
        assert!(format_uptime(u64::MAX).contains('d'));
    }
}
