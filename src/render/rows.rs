//! Row assembly: turning the snapshot's rows into styled lines, and arranging
//! those lines into columns. Nothing here reads the environment or the system.

use super::colour::Colour;
use super::layout::{split_cells, width, wrap};
use super::terminal::{Palette, capacity, colour_style, display, format_uptime, plain};
use super::theme::Role;
use crate::model::*;
use crate::module::ModuleId;
use anstyle::Style;

pub(super) struct Span {
    text: String,
    style: Style,
}

pub(super) struct Line {
    spans: Vec<Span>,
}

impl Line {
    fn new() -> Self {
        Self { spans: Vec::new() }
    }

    fn push(&mut self, text: impl Into<String>, style: Style) {
        let text = text.into();
        if !text.is_empty() {
            self.spans.push(Span { text, style });
        }
    }
}

pub(super) fn emit(line: &Line) -> String {
    // Trim trailing whitespace span by span, so a styled run of spaces cannot
    // leave escape codes stranded at the end of a line.
    let mut end = line.spans.len();
    while end > 0 && line.spans[end - 1].text.trim_end_matches(' ').is_empty() {
        end -= 1;
    }
    let mut output = String::new();
    for (index, span) in line.spans[..end].iter().enumerate() {
        let text = if index + 1 == end {
            span.text.trim_end_matches(' ')
        } else {
            span.text.as_str()
        };
        output.push_str(&super::theme::styled(text, span.style));
    }
    output
}

pub(super) fn joined(lines: &[Line]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(&emit(line));
        out.push('\n');
    }
    out
}

/// Title, rule, rows and swatches — the arrangement classic and vertical share.
pub(super) fn column_lines(
    title: &str,
    rows: &[Row],
    label_width: usize,
    separator: &str,
    columns: usize,
    decor: bool,
    palette: &Palette<'_>,
) -> Vec<Line> {
    let title_style = palette.role(Role::Title);
    let label_style = palette.role(Role::Label);
    let value_style = palette.role(Role::Value);
    let divider_style = palette.role(Role::Divider);
    let emphasis_style = palette.role(Role::Emphasis);

    let mut lines = title_lines(title, columns, title_style);
    if decor {
        let mut line = Line::new();
        line.push(divider_rule(title, columns, palette.unicode), divider_style);
        lines.push(line);
    }

    for row in rows {
        row_lines(
            &mut lines,
            row,
            label_width,
            separator,
            columns,
            RowStyles {
                label: label_style,
                value: value_style,
                emphasis: emphasis_style,
            },
        );
    }

    if decor {
        let mut line = Line::new();
        line.push(divider_rule(title, columns, palette.unicode), divider_style);
        lines.push(line);
        if !palette.theme.swatches.is_empty() {
            lines.push(swatch_line(palette, columns));
        }
    }
    lines
}

/// `Label: value`, unaligned, for the width a column cannot fit in.
pub(super) fn minimal_lines(
    title: &str,
    rows: &[Row],
    columns: usize,
    palette: &Palette<'_>,
) -> Vec<Line> {
    let title_style = palette.role(Role::Title);
    let label_style = palette.role(Role::Label);
    let value_style = palette.role(Role::Value);
    let emphasis_style = palette.role(Role::Emphasis);

    let mut lines = title_lines(title, columns, title_style);
    for row in rows {
        // Minimal owns its own punctuation and does not pad the label, so the
        // shared composer produces `Label: value`.
        row_lines(
            &mut lines,
            row,
            width(&row.label),
            ": ",
            columns,
            RowStyles {
                label: label_style,
                value: value_style,
                emphasis: emphasis_style,
            },
        );
    }
    lines
}

pub(super) struct RowStyles {
    label: Style,
    value: Style,
    emphasis: Style,
}

/// Lay one row out into `lines`. Both arrangements with a label column use
/// this: `column_lines` pads the label to the column and takes the configured
/// separator, `minimal_lines` passes the label's own width and `": "`.
pub(super) fn row_lines(
    lines: &mut Vec<Line>,
    row: &Row,
    label_width: usize,
    separator: &str,
    columns: usize,
    styles: RowStyles,
) {
    let padding = " ".repeat(label_width.saturating_sub(width(&row.label)));
    let prefix = format!("{}{padding}", row.label);
    let prefix_width = label_width + width(separator);
    let continuation = " ".repeat(prefix_width);
    // Keep a readable continuation indent whenever enough width exists.
    if columns > prefix_width + 2 {
        for (index, chunk) in wrap(&row.value, columns - prefix_width)
            .into_iter()
            .enumerate()
        {
            let mut line = Line::new();
            if index == 0 {
                line.push(prefix.clone(), styles.label);
                line.push(separator, Style::new());
            } else {
                line.push(continuation.clone(), Style::new());
            }
            push_value(&mut line, &chunk, styles.value, styles.emphasis);
            lines.push(line);
        }
    } else {
        // Too narrow to hold the column: wrap the whole row and style each
        // fragment by what it actually is. A line break can land inside the
        // label, inside the separator, or inside the value, so the fragment is
        // split at the boundaries rather than guessed from where the line
        // happens to start.
        let text = format!("{prefix}{separator}{}", row.value);
        let separator_end = label_width + width(separator);
        let mut consumed = 0;
        for chunk in wrap(&text, columns) {
            let mut line = Line::new();
            let mut rest = chunk.as_str();
            let mut at = consumed;
            if at < label_width {
                let (head, tail) = split_cells(rest, label_width - at);
                line.push(head, styles.label);
                at += width(head);
                rest = tail;
            }
            if at < separator_end {
                let (head, tail) = split_cells(rest, separator_end - at);
                line.push(head, Style::new());
                rest = tail;
            }
            if !rest.is_empty() {
                push_value(&mut line, rest, styles.value, styles.emphasis);
            }
            consumed += width(&chunk);
            lines.push(line);
        }
    }
}

/// A value run, with the `~` that marks an estimate given the emphasis role.
pub(super) fn push_value(line: &mut Line, text: &str, value_style: Style, emphasis_style: Style) {
    match text.strip_prefix("~ ") {
        Some(rest) => {
            line.push("~ ", emphasis_style);
            line.push(rest, value_style);
        }
        None => line.push(text, value_style),
    }
}

pub(super) fn title_lines(title: &str, columns: usize, style: Style) -> Vec<Line> {
    wrap(title, columns)
        .into_iter()
        .map(|chunk| {
            let mut line = Line::new();
            line.push(chunk, style);
            line
        })
        .collect()
}

pub(super) fn divider_rule(title: &str, columns: usize, unicode: bool) -> String {
    let glyph = if unicode { "─" } else { "-" };
    glyph.repeat(width(title).clamp(12, 40).min(columns))
}

pub(super) fn swatch_line(palette: &Palette<'_>, columns: usize) -> Line {
    let dot = if palette.unicode { "●" } else { "o" };
    let count = columns.div_ceil(2).min(8).min(palette.theme.swatches.len());
    let mut line = Line::new();
    for (index, colour) in palette.theme.swatches.iter().take(count).enumerate() {
        if index > 0 {
            line.push(" ", Style::new());
        }
        line.push(dot, colour_style(Some(*colour), palette.depth));
    }
    line
}

pub(super) fn logo_line(
    palette: &Palette<'_>,
    colours: &[Option<Colour>],
    index: usize,
    art: &str,
) -> String {
    let style = colour_style(colours.get(index).copied().flatten(), palette.depth);
    super::theme::styled(art, style)
}

pub(super) fn trim_end(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

/// One information row before layout: the label to show and the composed value.
pub(super) struct Row {
    pub(super) label: String,
    pub(super) value: String,
}

/// The rows to draw, in the order the user asked for. A module that is not in
/// the selection produces no row at all. Labels are custom user input, so they
/// are sanitised exactly like values.
pub(super) fn field_rows(
    snapshot: &SystemSnapshot,
    modules: &[ModuleId],
    labels: &[(ModuleId, String)],
    unicode: bool,
) -> Vec<Row> {
    modules
        .iter()
        .map(|&module| Row {
            label: plain(
                labels
                    .iter()
                    .find(|(id, _)| *id == module)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_else(|| module.label().to_owned())
                    .as_str(),
                unicode,
            ),
            value: plain(&value_for(snapshot, module), unicode),
        })
        .collect()
}

/// The composed value for one module. One arm per module: adding a module is a
/// compile error here rather than a row that silently renders nothing.
pub(super) fn value_for(snapshot: &SystemSnapshot, module: ModuleId) -> String {
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
        ModuleId::Gpu => display(&snapshot.gpu, |gpu| {
            gpu.devices
                .iter()
                .map(|device| match (&device.name, &device.vendor) {
                    (Some(name), _) => name.clone(),
                    // Without a model name, the vendor and the platform id are
                    // what the platform actually said.
                    (None, Some(vendor)) => match &device.id {
                        Some(id) => format!("{vendor} ({id})"),
                        None => vendor.clone(),
                    },
                    (None, None) => device
                        .id
                        .clone()
                        .unwrap_or_else(|| "unknown adapter".to_owned()),
                })
                .collect::<Vec<_>>()
                .join(", ")
        }),
        ModuleId::Display => display(&snapshot.display, |info| {
            info.displays
                .iter()
                .map(|unit| {
                    let mut text = format!("{}x{}", unit.width, unit.height);
                    if let Some(hz) = unit.refresh_hz {
                        text.push_str(&format!(" @{hz}Hz"));
                    }
                    match unit.kind {
                        DisplayKind::BuiltIn => text.push_str(" (built-in)"),
                        DisplayKind::Unknown => {}
                    }
                    if info.displays.len() > 1 && unit.primary == Some(true) {
                        text.push_str(" (primary)");
                    }
                    match &unit.name {
                        Some(name) => format!("{name} {text}"),
                        None => text,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        }),
        ModuleId::Packages => display(&snapshot.packages, |info| {
            info.managers
                .iter()
                .map(|manager| format!("{} {}", manager.name, manager.count))
                .collect::<Vec<_>>()
                .join(", ")
        }),
        ModuleId::Desktop => display(&snapshot.desktop, |info| match info.session {
            SessionType::X11 => format!("{} (X11)", info.name),
            SessionType::Wayland => format!("{} (Wayland)", info.name),
            SessionType::Tty => format!("{} (tty)", info.name),
            SessionType::Unknown => info.name.clone(),
        }),
        ModuleId::Locale => display(&snapshot.locale, |info| {
            match (&info.process, &info.system) {
                (Some(process), Some(system)) if process != system => {
                    format!(
                        "{process} ({}), system {system}",
                        info.process_source.as_deref().unwrap_or("env")
                    )
                }
                (Some(process), _) => {
                    format!(
                        "{process} ({})",
                        info.process_source.as_deref().unwrap_or("env")
                    )
                }
                (None, Some(system)) => format!("system {system}"),
                (None, None) => "Unavailable".to_owned(),
            }
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
        ModuleId::Battery => display(&snapshot.battery, |battery| {
            let percents = battery
                .units
                .iter()
                .map(|unit| match unit.percent {
                    Some(percent) => format!("{percent}%"),
                    None => "?".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" / ");
            // States that agree are named once; a machine whose batteries
            // disagree says so rather than picking one.
            let mut states: Vec<&str> = Vec::new();
            for unit in &battery.units {
                let state = match unit.state {
                    BatteryState::Charging => "charging",
                    BatteryState::Discharging => "discharging",
                    BatteryState::Full => "full",
                    BatteryState::Unknown => "unknown",
                };
                if !states.contains(&state) {
                    states.push(state);
                }
            }
            format!("{percents} ({})", states.join(", "))
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
