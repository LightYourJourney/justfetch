use super::{
    layout::{sanitize, width, wrap},
    logo, theme,
};
use crate::model::*;
use anstyle::AnsiColor;
use std::ffi::OsStr;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub columns: usize,
    pub color: bool,
    pub unicode: bool,
    pub logo: bool,
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

pub fn render(snapshot: &SystemSnapshot, options: Options) -> String {
    let os = snapshot.os.value();
    let logo = logo::select(os.and_then(|os| os.id.as_deref()), os.map(|os| &os.family));
    render_with_logo(snapshot, options, &logo)
}

/// Explicit logo input also exercises shorter and taller custom fixtures.
pub fn render_with_logo(snapshot: &SystemSnapshot, options: Options, logo: &logo::Logo) -> String {
    let columns = options.columns.clamp(1, 4096);
    let logo_lines: Vec<&str> = logo.art.lines().collect();
    let logo_width = logo_lines.iter().map(|l| width(l)).max().unwrap_or(0);
    let show_logo = options.logo && columns >= logo_width + 3 + 36;
    let info_columns = if show_logo {
        columns - logo_width - 3
    } else {
        columns
    };
    let username = display(&snapshot.identity.username, Clone::clone);
    let hostname = display(&snapshot.identity.hostname, Clone::clone);
    let title = plain(&format!("{username}@{hostname}"), options.unicode);
    let divider = if options.unicode { "─" } else { "-" }
        .repeat(width(&title).clamp(12, 40).min(info_columns));
    let mut rows = wrap(&title, info_columns);
    rows.push(divider.clone());
    let os = display(&snapshot.os, |os| match &os.version {
        Some(version) if !os.name.contains(version) => format!("{} {version}", os.name),
        _ => os.name.clone(),
    });
    let environment = match snapshot.environment {
        Environment::Native => "",
        Environment::Wsl { version: Some(2) } => " (WSL2)",
        Environment::Wsl { .. } => " (WSL)",
    };
    let host = display(&snapshot.host, |host| match host.kind {
        HostKind::DeviceModel => host.name.clone(),
        HostKind::Hostname => format!("{} (hostname)", host.name),
    });
    let kernel = display(&snapshot.kernel, Clone::clone);
    let uptime = display(&snapshot.uptime, |v| format_uptime(v.seconds));
    let cpu = display(&snapshot.cpu, |cpu| {
        format!("{} ({} logical)", cpu.name, cpu.logical_cores)
    });
    let memory = display(&snapshot.memory, |mem| {
        capacity(mem.used_bytes, mem.total_bytes)
    });
    let swap = display(&snapshot.swap, |swap| {
        if swap.total_bytes == 0 {
            "none".to_owned()
        } else {
            capacity(swap.used_bytes, swap.total_bytes)
        }
    });
    let disk = display(&snapshot.disk, |disk| {
        format!(
            "{} on {}",
            capacity(disk.used_bytes, disk.total_bytes),
            disk.path
        )
    });
    let shell = display(&snapshot.shell, |shell| match shell.kind {
        ShellKind::CurrentParent => shell.name.clone(),
        ShellKind::ConfiguredDefault => format!("{} (default)", shell.name),
    });
    let terminal = display(&snapshot.terminal, |terminal| match &terminal.version {
        Some(version) => format!("{} {version}", terminal.name),
        None => terminal.name.clone(),
    });
    let local_ip = display(&snapshot.local_ip, |local_ip| {
        local_ip
            .addresses
            .iter()
            .map(|entry| match &entry.interface {
                Some(interface) => format!("{} ({interface})", entry.address),
                None => entry.address.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    });
    let values = [
        format!("{os}{environment}"),
        host,
        kernel,
        uptime,
        cpu,
        memory,
        swap,
        disk,
        shell,
        terminal,
        local_ip,
    ];
    for (key, value) in FIELD_KEYS.into_iter().zip(values) {
        let value = plain(&value, options.unicode);
        let prefix = field_prefix(key);
        // Keep a readable continuation indent whenever enough width exists.
        if info_columns > 10 {
            let chunks = wrap(&value, info_columns - KEY_PREFIX_LEN);
            for (index, chunk) in chunks.into_iter().enumerate() {
                rows.push(format!(
                    "{}{chunk}",
                    if index == 0 {
                        prefix.as_str()
                    } else {
                        CONTINUATION
                    }
                ));
            }
        } else {
            rows.extend(wrap(&format!("{prefix}{value}"), info_columns));
        }
    }
    rows.push(divider);
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
    let row_count = rows.len().max(if show_logo { logo_lines.len() } else { 0 });
    for index in 0..row_count {
        if show_logo {
            let line = logo_lines.get(index).copied().unwrap_or("");
            output.push_str(&theme::accent(line, logo.color, options.color));
            output.push_str(&" ".repeat(logo_width - width(line) + 3));
        }
        if let Some(line) = rows.get(index) {
            if index == 0 {
                output.push_str(&theme::accent(line, logo.color, options.color));
            } else if index > 1
                && line.len() >= KEY_PREFIX_LEN
                && line.is_char_boundary(KEY_PREFIX_LEN)
                && fields_key(&line[..KEY_PREFIX_LEN])
            {
                output.push_str(&theme::accent(
                    &line[..KEY_PREFIX_LEN],
                    logo.color,
                    options.color,
                ));
                output.push_str(&line[KEY_PREFIX_LEN..]);
            } else {
                output.push_str(line);
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

/// Field keys in display order. The styling pass reads the same list, so the
/// rows and the keys they are styled by cannot disagree.
const FIELD_KEYS: [&str; 11] = [
    "OS", "Host", "Kernel", "Uptime", "CPU", "Memory", "Swap", "Disk", "Shell", "Term", "IP",
];

/// A field row opens with the key column, so this is how many bytes of a
/// composed row the styling pass replaces with a styled prefix.
const KEY_PREFIX_LEN: usize = 8;

/// Wrapped continuation rows indent by exactly the key column.
const CONTINUATION: &str = "        ";

fn field_prefix(key: &str) -> String {
    format!("{key:<6}  ")
}

fn fields_key(prefix: &str) -> bool {
    FIELD_KEYS.contains(&prefix.trim())
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
    /// The styling pass slices composed rows by `KEY_PREFIX_LEN`, so the
    /// constant has to keep matching what `field_prefix` actually produces.
    #[test]
    fn key_column_width_matches_the_styled_slice() {
        assert_eq!(CONTINUATION.len(), KEY_PREFIX_LEN);
        for key in FIELD_KEYS {
            assert_eq!(field_prefix(key).len(), KEY_PREFIX_LEN, "{key}");
        }
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
