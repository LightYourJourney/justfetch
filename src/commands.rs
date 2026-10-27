//! The `init` and `doctor` subcommands, and the text they print.
//!
//! They live apart from `config` because the two change for different reasons:
//! `config` follows the file format and the precedence rules, this module
//! follows what the commands say.

use crate::cli::Cli;
use crate::config::{ConfigError, resolve, user_config_path};
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
                    "layout: logo {}, colour {}, accent {}, separator {:?}",
                    config.logo,
                    config.color,
                    config
                        .accent
                        .map_or_else(|| "logo".to_owned(), |accent| format!("{accent:?}")),
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
    findings
}

/// What `init` writes. Every setting is shown at its default so the file can be
/// edited by deleting lines rather than by looking the format up.
pub const INIT_TEMPLATE: &str = r#"# justfetch configuration
#
# Every setting is optional. The values below are the built-in defaults, so
# deleting a line keeps the behaviour it describes.

# Modules to show, in display order. Identifiers come from `justfetch --help`.
modules = ["os", "host", "kernel", "uptime", "cpu", "memory", "swap", "disk", "shell", "terminal", "local_ip"]

[layout]
# "auto" shows a logo when the terminal is wide enough; "none" never does.
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
            logo: None,
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
        assert_eq!(parsed.modules, crate::module::ModuleId::ALL.to_vec());
        assert_eq!(parsed.logo, crate::config::LogoMode::Auto);
        assert!(parsed.color);
        assert_eq!(parsed.separator, crate::config::DEFAULT_SEPARATOR);
        assert!(parsed.labels.is_empty());
    }
}
