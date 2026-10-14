use crate::cli::{Cli, Format, Logo};

/// Effective v0.1 options. No configuration files are read.
pub struct Config {
    pub format: Format,
    pub logo: bool,
    pub no_color: bool,
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        Self {
            format: cli.format,
            logo: cli.logo == Logo::Auto,
            no_color: cli.no_color,
        }
    }
}

impl Config {
    pub fn terminal_options(&self) -> crate::render::terminal::Options {
        use std::io::IsTerminal;
        let tty = std::io::stdout().is_terminal();
        let term = std::env::var("TERM").ok();
        let no_color = std::env::var_os("NO_COLOR");
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()));
        let unicode = term.as_deref() != Some("dumb")
            && locale.is_none_or(|locale| {
                let locale = locale.to_ascii_lowercase();
                locale.contains("utf-8") || locale.contains("utf8")
            });
        crate::render::terminal::Options {
            columns: crate::platform::terminal_columns().unwrap_or(80),
            color: crate::render::terminal::color_allowed(
                tty,
                self.no_color,
                no_color.as_deref(),
                term.as_deref(),
            ) && crate::platform::supports_ansi(),
            unicode,
            logo: self.logo,
        }
    }
}
