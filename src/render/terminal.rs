use super::{
    colour::{Colour, Depth},
    layout::{Layout, sanitize, split_cells, width, wrap},
    logo,
    theme::{self, Role, Theme},
};
use crate::model::*;
use crate::module::ModuleId;
use anstyle::Style;
use std::ffi::OsStr;

#[derive(Debug)]
pub struct Options<'a> {
    pub columns: usize,
    /// What the terminal can be told about colour. `None` renders plain text.
    pub depth: Depth,
    pub unicode: bool,
    /// Whether artwork may be drawn at all. The layout still decides whether
    /// there is room for it.
    pub logo: bool,
    /// The user named this artwork explicitly, so a narrow terminal stacks it
    /// above the information instead of dropping it.
    pub logo_explicit: bool,
    pub layout: Layout,
    /// Which rows to draw, in this order.
    pub modules: &'a [ModuleId],
    /// Custom labels, by module. Anything absent keeps its default label.
    pub labels: &'a [(ModuleId, String)],
    /// Text between the label column and the value.
    pub separator: &'a str,
    pub theme: &'a Theme,
    /// Overrides the accent the theme would otherwise take from its logo.
    pub accent: Option<Colour>,
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

/// Columns kept between artwork and the information beside it.
const LOGO_GAP: usize = 3;
/// Columns of information required before artwork is given room beside it.
const MIN_INFO_BESIDE_LOGO: usize = 36;

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

/// The artwork the detected system calls for. One place decides it, so the
/// library entry point and the CLI cannot disagree.
pub fn auto_logo(snapshot: &SystemSnapshot) -> logo::Logo {
    let os = snapshot.os.value();
    logo::select(os.and_then(|os| os.id.as_deref()), os.map(|os| &os.family))
}

pub fn render(snapshot: &SystemSnapshot, options: Options<'_>) -> String {
    render_with_logo(snapshot, options, &auto_logo(snapshot))
}

/// Explicit logo input also exercises shorter and taller custom fixtures.
pub fn render_with_logo(
    snapshot: &SystemSnapshot,
    options: Options<'_>,
    logo: &logo::Logo,
) -> String {
    let columns = options.columns.clamp(1, 4096);
    let theme = options.theme;
    // The user's accent overrides the theme; the theme's logo colour is the
    // fallback for roles neither the user nor the theme names.
    let accent = options.accent;
    let fallback = theme.logo.or(logo.color);
    let title = plain(
        &format!(
            "{}@{}",
            display(&snapshot.identity.username, Clone::clone),
            display(&snapshot.identity.hostname, Clone::clone)
        ),
        options.unicode,
    );
    let rows = field_rows(snapshot, options.modules, options.labels, options.unicode);
    let separator = plain(options.separator, options.unicode);
    // The classic column is as wide as the built-in labels need, or as wide as
    // a custom label on a visible row makes it. It deliberately does not shrink
    // to fit a subset: the same row should align the same way whatever else is
    // shown. Compact and minimal lay themselves out from the rows they show.
    let full_label_width = ModuleId::ALL
        .iter()
        .map(|module| width(module.label()))
        .chain(rows.iter().map(|row| width(&row.label)))
        .max()
        .unwrap_or(0);
    let shown_label_width = rows.iter().map(|row| width(&row.label)).max().unwrap_or(0);

    let palette = Palette {
        theme,
        depth: options.depth,
        accent,
        fallback,
        unicode: options.unicode,
    };
    // Artwork is user input too: in a non-UTF-8 locale it is downgraded exactly
    // like the labels and values, so a custom CJK logo cannot arrive as mojibake
    // while every other string is ASCII.
    let logo_lines: Vec<String> = logo
        .lines()
        .into_iter()
        .map(|line| plain(line, options.unicode))
        .collect();
    let logo_width = logo_lines.iter().map(|line| width(line)).max().unwrap_or(0);

    // An explicitly chosen logo is never silently discarded. When classic has
    // no room beside the information, the artwork moves above it — the same
    // arrangement as `vertical` — rather than disappearing.
    let mut layout = options.layout;
    if layout == Layout::Classic
        && options.logo
        && options.logo_explicit
        && columns < logo_width + 3 + 36
        && columns >= logo_width
    {
        layout = Layout::Vertical;
    }

    match layout {
        Layout::Classic => {
            let show_logo = options.logo && columns >= logo_width + LOGO_GAP + MIN_INFO_BESIDE_LOGO;
            let logo_colours = theme.logo_colours(logo.color, logo_lines.len());
            let info_columns = if show_logo {
                columns - logo_width - LOGO_GAP
            } else {
                columns
            };
            let lines = column_lines(
                &title,
                &rows,
                full_label_width,
                &separator,
                info_columns,
                decor_fits(columns),
                &palette,
            );
            let mut out = String::new();
            let height = lines
                .len()
                .max(if show_logo { logo_lines.len() } else { 0 });
            for index in 0..height {
                if show_logo {
                    let art = logo_lines.get(index).map(String::as_str).unwrap_or("");
                    out.push_str(&logo_line(&palette, &logo_colours, index, art));
                    out.push_str(&" ".repeat(logo_width - width(art) + 3));
                }
                match lines.get(index) {
                    Some(line) => out.push_str(&emit(line)),
                    // Drop the padding left by artwork that runs longer than
                    // the information.
                    None => trim_end(&mut out),
                }
                out.push('\n');
            }
            out
        }
        Layout::Vertical => {
            let show_logo = options.logo && columns >= logo_width;
            let logo_colours = theme.logo_colours(logo.color, logo_lines.len());
            let lines = column_lines(
                &title,
                &rows,
                full_label_width,
                &separator,
                columns,
                decor_fits(columns),
                &palette,
            );
            let mut out = String::new();
            if show_logo {
                // Centre the block, not each line: centring a line by its own
                // width compresses an asymmetric drawing — the arch's
                // staircase loses its slope.
                for (index, art) in logo_lines.iter().enumerate() {
                    out.push_str(&" ".repeat((columns - logo_width) / 2));
                    out.push_str(&logo_line(&palette, &logo_colours, index, art));
                    out.push('\n');
                }
                out.push('\n');
            }
            for line in &lines {
                out.push_str(&emit(line));
                out.push('\n');
            }
            out
        }
        Layout::Compact => {
            let lines = column_lines(
                &title,
                &rows,
                shown_label_width,
                &separator,
                columns,
                false,
                &palette,
            );
            joined(&lines)
        }
        Layout::Minimal => joined(&minimal_lines(&title, &rows, columns, &palette)),
    }
}

/// Whether the rules and swatches fit. Below this width they would crowd out
/// the information they are meant to frame.
fn decor_fits(columns: usize) -> bool {
    columns >= 24
}

/// The resolved styling a render needs: which theme, at which depth, with the
/// user's accent on top and the artwork colour underneath.
struct Palette<'a> {
    theme: &'a Theme,
    depth: Depth,
    accent: Option<Colour>,
    fallback: Option<Colour>,
    unicode: bool,
}

impl Palette<'_> {
    fn role(&self, role: Role) -> Style {
        self.theme
            .style(role, self.accent, self.fallback, self.depth)
    }
}

pub(crate) fn colour_style(colour: Option<Colour>, depth: Depth) -> Style {
    match colour.and_then(|colour| colour.resolve(depth)) {
        Some(color) => Style::new().fg_color(Some(color)).bold(),
        None => Style::new(),
    }
}

/// One coloured run of text. Styling is attached after all widths are known, so
/// an escape can never shift a column.
struct Span {
    text: String,
    style: Style,
}

struct Line {
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

fn emit(line: &Line) -> String {
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
        output.push_str(&theme::styled(text, span.style));
    }
    output
}

fn joined(lines: &[Line]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(&emit(line));
        out.push('\n');
    }
    out
}

/// Title, rule, rows and swatches — the arrangement classic and vertical share.
fn column_lines(
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
fn minimal_lines(title: &str, rows: &[Row], columns: usize, palette: &Palette<'_>) -> Vec<Line> {
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

struct RowStyles {
    label: Style,
    value: Style,
    emphasis: Style,
}

/// Lay one row out into `lines`. Both arrangements with a label column use
/// this: `column_lines` pads the label to the column and takes the configured
/// separator, `minimal_lines` passes the label's own width and `": "`.
fn row_lines(
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
fn push_value(line: &mut Line, text: &str, value_style: Style, emphasis_style: Style) {
    match text.strip_prefix("~ ") {
        Some(rest) => {
            line.push("~ ", emphasis_style);
            line.push(rest, value_style);
        }
        None => line.push(text, value_style),
    }
}

fn title_lines(title: &str, columns: usize, style: Style) -> Vec<Line> {
    wrap(title, columns)
        .into_iter()
        .map(|chunk| {
            let mut line = Line::new();
            line.push(chunk, style);
            line
        })
        .collect()
}

fn divider_rule(title: &str, columns: usize, unicode: bool) -> String {
    let glyph = if unicode { "─" } else { "-" };
    glyph.repeat(width(title).clamp(12, 40).min(columns))
}

fn swatch_line(palette: &Palette<'_>, columns: usize) -> Line {
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

fn logo_line(palette: &Palette<'_>, colours: &[Option<Colour>], index: usize, art: &str) -> String {
    let style = colour_style(colours.get(index).copied().flatten(), palette.depth);
    theme::styled(art, style)
}

fn trim_end(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

/// One information row before layout: the label to show and the composed value.
struct Row {
    label: String,
    value: String,
}

/// The rows to draw, in the order the user asked for. A module that is not in
/// the selection produces no row at all. Labels are custom user input, so they
/// are sanitised exactly like values.
fn field_rows(
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
    #[test]
    fn uptime_units() {
        assert_eq!(format_uptime(59), "59s");
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(3660), "1h 1m");
        assert_eq!(format_uptime(90061), "1d 1h 1m");
        assert!(format_uptime(u64::MAX).contains('d'));
    }
}
