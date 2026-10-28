//! The `init`, `doctor` and `themes` subcommands, and the text they print.
//!
//! They live apart from `config` because they change for different reasons:
//! `config` follows the file format and the precedence rules, this module
//! follows what the commands say.

use crate::cli::Cli;
use crate::config::{ConfigError, resolve, user_config_path};
use crate::render::{
    colour::Depth,
    layout::{width, wrap},
    terminal::colour_style,
    theme::{self, Role, styled},
};
use std::path::PathBuf;

/// One line of `doctor` output plus whether it is a problem.
pub struct Finding {
    pub problem: bool,
    pub text: String,
}

/// Check what can actually be checked: the resolved path, whether the file
/// exists, whether it parses, and what the effective settings became.
pub fn diagnose(cli: &Cli) -> Vec<Finding> {
    let mut findings = Vec::new();
    fn problem(findings: &mut Vec<Finding>, text: String) {
        findings.push(Finding {
            problem: true,
            text,
        });
    }

    match user_config_path() {
        Some(path) if path.exists() => findings.push(Finding {
            problem: false,
            text: format!("user config: {} (found)", path.display()),
        }),
        Some(path) => findings.push(Finding {
            problem: false,
            text: format!(
                "user config: {} (not present; {} apply)",
                path.display(),
                if cli.config.is_some() {
                    "the --config file and the built-in defaults"
                } else {
                    "the built-in defaults"
                }
            ),
        }),
        // A run with `--config`, or with no file at all, works without a home
        // directory, so its absence is reported but is not a failure.
        None => findings.push(Finding {
            problem: false,
            text: "user config: no home directory in the environment, so only --config and the built-in defaults apply".to_owned(),
        }),
    }
    match &cli.config {
        Some(path) if path.exists() => findings.push(Finding {
            problem: false,
            text: format!("--config: {} (found)", path.display()),
        }),
        // A normal run stops here too, so there is nothing further to resolve
        // and no second error line repeating this one.
        Some(path) => {
            problem(
                &mut findings,
                format!("--config: {} (file not found)", path.display()),
            );
            return findings;
        }
        None => {}
    }

    match resolve(cli) {
        Ok(config) => {
            findings.push(Finding {
                problem: false,
                text: format!(
                    "modules: {} selected, in this order: {}",
                    config.modules.len(),
                    config
                        .modules
                        .iter()
                        .map(|module| module.id())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
            findings.push(Finding {
                problem: false,
                text: format!(
                    "layout: {} layout, logo {}, theme {}, colour {}, accent {}, separator {:?}",
                    config.layout,
                    if config.logo_art.is_some() {
                        "custom file".to_owned()
                    } else {
                        config.logo.to_string()
                    },
                    config.theme.name,
                    config.color,
                    config
                        .accent
                        .map_or_else(|| "logo".to_owned(), |accent| accent.to_string()),
                    config.separator
                ),
            });
            if crate::render::layout::sanitize(&config.separator) != config.separator {
                problem(
                    &mut findings,
                    "layout.separator contains characters that are stripped when rendering"
                        .to_owned(),
                );
            }
            for (module, label) in &config.labels {
                if crate::render::layout::sanitize(label) != *label {
                    problem(
                        &mut findings,
                        format!(
                            "labels.{} contains characters that are stripped when rendering",
                            module.id()
                        ),
                    );
                } else {
                    findings.push(Finding {
                        problem: false,
                        text: format!("label {}: {label}", module.id()),
                    });
                }
            }
        }
        Err(error) => problem(&mut findings, format!("configuration error: {error}")),
    }
    // Paths and configuration values are user input, and each finding is one
    // line, so a newline inside one must not become a second line of output.
    for finding in &mut findings {
        finding.text = crate::render::layout::sanitize(&finding.text);
    }
    findings
}

/// What `init` writes. Every setting is shown at its default so the file can be
/// edited by deleting lines rather than by looking the format up.
pub const INIT_TEMPLATE: &str = r#"# justfetch configuration
#
# Every setting is optional. The values below are the built-in defaults, so
# deleting a line keeps the behaviour it describes.

# Modules to show, in display order. Identifiers come from `justfetch --help`.
modules = ["os", "host", "kernel", "uptime", "cpu", "gpu", "memory", "swap", "disk", "battery", "shell", "terminal", "local_ip"]

[theme]
# A built-in theme, or `file = "path.toml"` for one of your own.
# Themes: default, catppuccin-mocha, nord, tokyo-night, gruvbox, monochrome.
name = "default"

[layout]
# Layout: classic, compact, minimal, vertical.
name = "classic"
# "auto" follows the detected distribution, "none" hides the artwork, or name
# a built-in logo: macos, windows, linux, arch, debian, ubuntu, fedora, gentoo,
# opensuse, nixos, fallback.
logo = "auto"
# Text between the label column and the value.
separator = "  "

[color]
# ANSI colour, only on a terminal that supports it.
enabled = true
# Accent for the title and labels. Omit to keep the logo's own colour.
#accent = "cyan"

#[labels]
# Rename a field. Keys are module ids; values are shown instead of the default.
#cpu = "Processor"
"#;

/// Write the default configuration. Never overwrites an existing file.
pub fn init(cli: &Cli) -> Result<PathBuf, ConfigError> {
    let path = match &cli.config {
        Some(path) => path.clone(),
        None => user_config_path().ok_or_else(|| {
            ConfigError::plain("no user config directory in the environment; pass --config PATH")
        })?,
    };
    if path.exists() {
        return Err(ConfigError::plain(format!(
            "{}: already exists, not overwriting",
            path.display()
        )));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| ConfigError::at(parent, format!("cannot create: {error}")))?;
    }
    std::fs::write(&path, INIT_TEMPLATE)
        .map_err(|error| ConfigError::at(&path, format!("cannot write: {error}")))?;
    Ok(path)
}

/// The built-in themes with a colour preview. Colour follows the same policy as
/// a normal run, so a redirected listing stays plain text.
pub fn themes(depth: Depth, unicode: bool, columns: usize) -> String {
    // Colour and width come from the same resolution a normal run uses, so
    // `--no-color`, a non-TTY, a configuration and a non-UTF-8 locale apply
    // here too.
    let dot = if unicode { "\u{25cf}" } else { "o" };
    let mut out = String::new();
    for line in wrap("Themes (`--theme NAME`, `--theme-file PATH`):", columns) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    let name_width = theme::names()
        .iter()
        .map(|name| name.len())
        .max()
        .unwrap_or(0);
    for name in theme::names() {
        let Some(theme) = theme::built_in(name) else {
            continue;
        };
        // Below this width the name column and a description cannot share a
        // line without overflowing, so the entry becomes a small block.
        let inline = columns >= name_width + 12;
        let indent = if inline { name_width + 2 } else { 0 };
        let body = columns.saturating_sub(indent).max(1);
        let chips: Vec<String> = theme
            .swatches
            .iter()
            .take(body.div_ceil(2))
            .map(|colour| styled(dot, colour_style(Some(*colour), depth)))
            .collect();
        // The preview has to preview: the name carries the theme's title
        // colour, and a sample follows the label, value, rule, emphasis and
        // artwork colours.
        let title_style = theme.style(Role::Title, None, theme.logo, depth);
        let block = if unicode { "██" } else { "##" };
        let rule = if unicode { "──" } else { "--" };
        // One list drives both what is drawn and how wide it is, so a change
        // to the sample cannot leave the fit check behind.
        let mut samples = vec![
            (Sample::Role(Role::Label), "Aa"),
            (Sample::Role(Role::Value), "Aa"),
            (Sample::Role(Role::Divider), rule),
            (Sample::Role(Role::Emphasis), "~"),
            (Sample::Logo, block),
        ];
        if theme.logo_secondary.is_some() {
            samples.push((Sample::Secondary, block));
        }
        let styled_parts: Vec<String> = samples
            .iter()
            .map(|(kind, text)| {
                let style = match kind {
                    Sample::Role(role) => theme.style(*role, None, theme.logo, depth),
                    Sample::Logo => colour_style(theme.logo, depth),
                    Sample::Secondary => colour_style(theme.logo_secondary, depth),
                };
                styled(text, style)
            })
            .collect();
        // Two spaces before the sample, one between its parts; the chips are
        // one cell each with a single space between them.
        let sample_width = 2
            + samples.iter().map(|(_, text)| width(text)).sum::<usize>()
            + samples.len().saturating_sub(1);
        let chips_width = chips.len().saturating_mul(2).saturating_sub(1);
        // `sample_width` already counts the two spaces it opens with, so the
        // separator before it is not added twice.
        let line_width = name_width + 2 + chips_width + sample_width;
        let sample = if inline && columns >= line_width {
            format!("  {}", styled_parts.join(" "))
        } else {
            String::new()
        };
        if inline {
            let padding = " ".repeat(name_width.saturating_sub(width(name)));
            out.push_str(&format!(
                "{}{padding}  {}{sample}\n",
                styled(name, title_style),
                chips.join(" ")
            ));
        } else {
            for part in wrap(name, columns) {
                out.push_str(&part);
                out.push('\n');
            }
            out.push_str(&chips.join(" "));
            out.push('\n');
        }
        for (index, line) in wrap_words(&theme.description, body).into_iter().enumerate() {
            if index == 0 && inline {
                out.push_str(&format!("{:name_width$}  {line}\n", ""));
            } else {
                out.push_str(&format!("{}{line}\n", " ".repeat(indent)));
            }
        }
        out.push('\n');
    }

    out
}

/// The same listing as a document, for `--format json`. A JSON request should
/// return JSON; prose on stdout would break whatever asked for it.
pub fn themes_json() -> String {
    let themes: Vec<_> = theme::names()
        .iter()
        .filter_map(|name| theme::built_in(name))
        .map(|theme| {
            serde_json::json!({
                "name": theme.name,
                "description": theme.description,
                "swatches": theme
                    .swatches
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    // Serialising a `Value` of strings cannot fail; an empty list is the safe
    // answer rather than a panic if it ever does.
    let mut text = serde_json::to_string_pretty(&themes).unwrap_or_else(|_| "[]".to_owned());
    text.push('\n');
    text
}

/// One part of the theme preview's sample, and which colour it demonstrates.
enum Sample {
    Role(Role),
    Logo,
    Secondary,
}

/// Break prose at spaces. `wrap` works on graphemes and is right for values,
/// which may have no spaces at all; a description is sentences, and splitting
/// one mid-word reads as a defect. A word longer than the line still falls back
/// to grapheme wrapping rather than overflowing.
fn wrap_words(text: &str, columns: usize) -> Vec<String> {
    let columns = columns.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if width(word) > columns {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut parts = wrap(word, columns);
            let last = parts.pop().unwrap_or_default();
            lines.extend(parts);
            current = last;
            continue;
        }
        if current.is_empty() {
            current.push_str(word);
        } else if width(&current) + 1 + width(word) <= columns {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Format;
    use std::path::Path;

    fn cli() -> Cli {
        Cli {
            command: None,
            no_color: false,
            color: false,
            format: Format::Text,
            modules: None,
            config: None,
            theme: None,
            theme_file: None,
            layout: None,
            logo: None,
            logo_file: None,
            privacy: false,
        }
    }

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jf-command-test-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prose_breaks_between_words_and_only_falls_back_inside_one() {
        assert_eq!(wrap_words("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap_words("one two three", 40), ["one two three"]);
        // A word that cannot fit is split, but only as a last resort.
        let parts = wrap_words("abcdefghij k", 4);
        assert_eq!(parts, ["abcd", "efgh", "ij k"]);
        assert_eq!(wrap_words("", 10), [""]);
    }

    #[test]
    fn the_init_template_parses_to_the_defaults() {
        let dir = tempdir("template");
        let path = write(&dir, "config.toml", INIT_TEMPLATE);
        let parsed = crate::config::resolve_with(
            &Cli {
                config: Some(path),
                ..cli()
            },
            None,
        )
        .unwrap();
        assert_eq!(parsed.modules, crate::module::ModuleId::DEFAULT.to_vec());
        assert_eq!(parsed.logo, crate::config::LogoChoice::Auto);
        assert!(parsed.color);
        assert_eq!(parsed.separator, crate::config::DEFAULT_SEPARATOR);
        assert!(parsed.labels.is_empty());
    }
}
