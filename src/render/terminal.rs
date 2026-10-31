use super::{
    colour::{Colour, Depth},
    layout::{Layout, sanitize, width},
    logo,
    theme::{Role, Theme},
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
pub(super) fn capacity(used: u64, total: u64) -> String {
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

pub(super) fn display<T>(detection: &Detection<T>, f: impl FnOnce(&T) -> String) -> String {
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

pub(super) fn plain(text: &str, unicode: bool) -> String {
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
pub(super) struct Palette<'a> {
    pub(super) theme: &'a Theme,
    pub(super) depth: Depth,
    accent: Option<Colour>,
    fallback: Option<Colour>,
    pub(super) unicode: bool,
}

impl Palette<'_> {
    pub(super) fn role(&self, role: Role) -> Style {
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
use super::rows::*;

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
