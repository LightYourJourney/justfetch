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
    let id = snapshot.os.value().and_then(|os| os.id.as_deref());
    let mut logo = logo::select(id);
    if logo.name == "fallback"
        && snapshot
            .os
            .value()
            .is_some_and(|os| os.family == OsFamily::Linux)
    {
        logo = logo::select(Some("linux"));
    }
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
        format!(
            "{:.2} / {:.2} GiB ({:.0}%)",
            mem.used_bytes as f64 / 1_073_741_824.0,
            mem.total_bytes as f64 / 1_073_741_824.0,
            mem.used_bytes as f64 / mem.total_bytes.max(1) as f64 * 100.0
        )
    });
    let shell = display(&snapshot.shell, |shell| match shell.kind {
        ShellKind::CurrentParent => shell.name.clone(),
        ShellKind::ConfiguredDefault => format!("{} (default)", shell.name),
    });
    let fields = [
        ("OS", format!("{os}{environment}")),
        ("Host", host),
        ("Kernel", kernel),
        ("Uptime", uptime),
        ("CPU", cpu),
        ("Memory", memory),
        ("Shell", shell),
    ];
    for (key, value) in fields {
        let text = format!("{key:<6}  {}", plain(&value, options.unicode));
        // Keep a readable continuation indent whenever enough width exists.
        if info_columns > 10 {
            let prefix = format!("{key:<6}  ");
            let chunks = wrap(&plain(&value, options.unicode), info_columns - 8);
            for (index, chunk) in chunks.into_iter().enumerate() {
                rows.push(format!(
                    "{}{chunk}",
                    if index == 0 {
                        prefix.as_str()
                    } else {
                        "        "
                    }
                ));
            }
        } else {
            rows.extend(wrap(&text, info_columns));
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
                && line.len() >= 8
                && line.is_char_boundary(8)
                && fields_key(&line[..8])
            {
                output.push_str(&theme::accent(&line[..8], logo.color, options.color));
                output.push_str(&line[8..]);
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

fn fields_key(prefix: &str) -> bool {
    matches!(
        prefix.trim(),
        "OS" | "Host" | "Kernel" | "Uptime" | "CPU" | "Memory" | "Shell"
    )
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
