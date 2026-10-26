//! Configuration: file format, locations, precedence and diagnostics.
//!
//! Precedence, tightest first: command-line arguments, an explicit `--config`
//! file, the user configuration file, built-in defaults. Nothing is ever
//! written to the user's directory unless `init` is asked for.

use crate::cli::{Cli, Format, Logo};
use crate::module::ModuleId;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

pub const DEFAULT_SEPARATOR: &str = "  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoMode {
    Auto,
    None,
}

/// The effective settings after precedence has been applied.
#[derive(Debug)]
pub struct Config {
    pub format: Format,
    pub privacy: bool,
    pub modules: Vec<ModuleId>,
    pub logo: LogoMode,
    pub color: bool,
    pub separator: String,
    pub labels: Vec<(ModuleId, String)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: Format::Text,
            privacy: false,
            modules: ModuleId::ALL.to_vec(),
            logo: LogoMode::Auto,
            color: true,
            separator: DEFAULT_SEPARATOR.to_owned(),
            labels: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ConfigError {
    path: Option<PathBuf>,
    message: String,
}

impl ConfigError {
    fn at(path: &Path, message: impl Into<String>) -> Self {
        Self {
            path: Some(path.to_path_buf()),
            message: message.into(),
        }
    }

    fn plain(message: impl Into<String>) -> Self {
        Self {
            path: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.message),
            None => f.write_str(&self.message),
        }
    }
}

// The file schema. `deny_unknown_fields` turns a misspelt key into an error
// that names it, rather than a setting that is silently ignored.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    modules: Option<Vec<String>>,
    layout: Option<LayoutSection>,
    color: Option<ColorSection>,
    labels: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutSection {
    logo: Option<String>,
    separator: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ColorSection {
    enabled: Option<bool>,
}

/// Where the user configuration lives.
#[cfg(not(windows))]
pub fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })?;
    Some(base.join("justfetch").join("config.toml"))
}

#[cfg(windows)]
pub fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)?;
    Some(base.join("justfetch").join("config.toml"))
}

fn read_file(path: &Path) -> Result<FileConfig, ConfigError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| ConfigError::at(path, format!("cannot be read: {error}")))?;
    toml::from_str(&text).map_err(|error| ConfigError::at(path, error.to_string()))
}

fn parse_modules(list: &[String], origin: &str) -> Result<Vec<ModuleId>, ConfigError> {
    let mut modules = Vec::with_capacity(list.len());
    for name in list {
        let module = ModuleId::parse(name).ok_or_else(|| {
            ConfigError::plain(format!(
                "{origin}: `{name}` is not a module id; expected one of {}",
                ModuleId::ALL.map(|m| m.id()).join(", ")
            ))
        })?;
        if modules.contains(&module) {
            return Err(ConfigError::plain(format!(
                "{origin}: `{name}` is listed twice"
            )));
        }
        modules.push(module);
    }
    if modules.is_empty() {
        return Err(ConfigError::plain(format!(
            "{origin}: the module list is empty; name at least one module or omit the setting"
        )));
    }
    Ok(modules)
}

impl FileConfig {
    fn apply(self, config: &mut Config, path: &Path) -> Result<(), ConfigError> {
        if let Some(list) = self.modules {
            config.modules = parse_modules(&list, &format!("{}: modules", path.display()))?;
        }
        if let Some(layout) = self.layout {
            if let Some(logo) = layout.logo {
                config.logo = match logo.trim().to_ascii_lowercase().as_str() {
                    "auto" => LogoMode::Auto,
                    "none" => LogoMode::None,
                    other => {
                        return Err(ConfigError::at(
                            path,
                            format!("layout.logo is `{other}`; expected `auto` or `none`"),
                        ));
                    }
                };
            }
            if let Some(separator) = layout.separator {
                if separator.is_empty() {
                    return Err(ConfigError::at(
                        path,
                        "layout.separator is empty; omit it for the default of two spaces",
                    ));
                }
                config.separator = separator;
            }
        }
        if let Some(color) = self.color
            && let Some(enabled) = color.enabled
        {
            config.color = enabled;
        }
        if let Some(labels) = self.labels {
            let mut parsed = Vec::with_capacity(labels.len());
            for (key, value) in labels {
                let module = ModuleId::parse(&key).ok_or_else(|| {
                    ConfigError::at(path, format!("labels.{key} is not a module id"))
                })?;
                // `cpu` and `CPU` are the same module; accepting both would let
                // one silently win.
                if parsed.iter().any(|(seen, _)| *seen == module) {
                    return Err(ConfigError::at(
                        path,
                        format!("labels.{key} names the same module as an earlier key"),
                    ));
                }
                if value.trim().is_empty() {
                    return Err(ConfigError::at(
                        path,
                        format!("labels.{key} is empty; omit it to keep the default label"),
                    ));
                }
                parsed.push((module, value));
            }
            config.labels = parsed;
        }
        Ok(())
    }
}

/// Apply the documented precedence and return the effective settings.
pub fn resolve(cli: &Cli) -> Result<Config, ConfigError> {
    resolve_with(cli, user_config_path().as_deref())
}

/// The same, with the user configuration path supplied. Tests pass `None` so
/// they never read whatever file the person running the suite happens to have.
pub fn resolve_with(cli: &Cli, user_config: Option<&Path>) -> Result<Config, ConfigError> {
    let mut config = Config::default();

    if let Some(path) = &cli.config {
        if !path.exists() {
            return Err(ConfigError::plain(format!(
                "--config {}: file not found",
                path.display()
            )));
        }
        read_file(path)?.apply(&mut config, path)?;
    } else if let Some(path) = user_config
        && path.exists()
    {
        read_file(path)?.apply(&mut config, path)?;
    }

    // Command line wins over every file.
    if let Some(list) = &cli.modules {
        let parts: Vec<String> = list
            .split(',')
            .map(|part| part.trim().to_owned())
            .filter(|part| !part.is_empty())
            .collect();
        config.modules = parse_modules(&parts, "the --modules argument")?;
    }
    if let Some(logo) = cli.logo {
        config.logo = match logo {
            Logo::Auto => LogoMode::Auto,
            Logo::None => LogoMode::None,
        };
    }
    if cli.no_color {
        config.color = false;
    }
    if cli.color {
        config.color = true;
    }
    config.format = cli.format;
    config.privacy = cli.privacy;
    Ok(config)
}

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
                "user config: {} (not present; built-in defaults apply)",
                path.display()
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
        Some(path) => problem(
            &mut findings,
            format!("--config: {} (file not found)", path.display()),
        ),
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
                    "layout: logo {:?}, colour {}, separator {:?}",
                    config.logo, config.color, config.separator
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

impl Config {
    pub fn terminal_options(&self) -> crate::render::terminal::Options<'_> {
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
            color: self.color
                && crate::render::terminal::color_allowed(
                    tty,
                    false,
                    no_color.as_deref(),
                    term.as_deref(),
                )
                && crate::platform::supports_ansi(),
            unicode,
            logo: self.logo == LogoMode::Auto,
            modules: &self.modules,
            labels: &self.labels,
            separator: &self.separator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        let dir = std::env::temp_dir().join(format!("jf-config-test-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_are_every_module_in_canonical_order() {
        let config = resolve_with(&cli(), None).unwrap();
        assert_eq!(config.modules, ModuleId::ALL.to_vec());
        assert_eq!(config.logo, LogoMode::Auto);
        assert!(config.color);
        assert_eq!(config.separator, DEFAULT_SEPARATOR);
        assert!(config.labels.is_empty());
        assert_eq!(config.format, Format::Text);
        assert!(!config.privacy);
    }

    #[test]
    fn the_init_template_parses_to_the_defaults() {
        let dir = tempdir("template");
        let path = write(&dir, "config.toml", INIT_TEMPLATE);
        let config = resolve_with(
            &Cli {
                config: Some(path),
                ..cli()
            },
            None,
        )
        .unwrap();
        assert_eq!(config.modules, ModuleId::ALL.to_vec());
        assert_eq!(config.logo, LogoMode::Auto);
        assert!(config.color);
        assert_eq!(config.separator, DEFAULT_SEPARATOR);
        assert!(config.labels.is_empty());
    }

    #[test]
    fn command_line_beats_the_file_which_beats_the_defaults() {
        let dir = tempdir("precedence");
        let path = write(
            &dir,
            "config.toml",
            "modules = [\"cpu\"]\n\n[layout]\nlogo = \"none\"\n\n[color]\nenabled = false\n",
        );
        // File over defaults.
        let from_file = resolve_with(
            &Cli {
                config: Some(path.clone()),
                ..cli()
            },
            None,
        )
        .unwrap();
        assert_eq!(from_file.modules, vec![ModuleId::Cpu]);
        assert_eq!(from_file.logo, LogoMode::None);
        assert!(!from_file.color);
        // Command line over the file.
        let from_cli = resolve_with(
            &Cli {
                config: Some(path),
                modules: Some("memory,os".to_owned()),
                logo: Some(Logo::Auto),
                color: true,
                ..cli()
            },
            None,
        )
        .unwrap();
        assert_eq!(from_cli.modules, vec![ModuleId::Memory, ModuleId::Os]);
        assert_eq!(from_cli.logo, LogoMode::Auto);
        assert!(from_cli.color);
    }

    #[test]
    fn the_user_file_is_used_when_no_explicit_one_is_given() {
        let dir = tempdir("user-file");
        let path = write(&dir, "user.toml", "modules = [\"uptime\"]\n");
        let config = resolve_with(&cli(), Some(&path)).unwrap();
        assert_eq!(config.modules, vec![ModuleId::Uptime]);
        // An explicit --config wins over the user file.
        let explicit = write(&dir, "explicit.toml", "modules = [\"shell\"]\n");
        let config = resolve_with(
            &Cli {
                config: Some(explicit),
                ..cli()
            },
            Some(&path),
        )
        .unwrap();
        assert_eq!(config.modules, vec![ModuleId::Shell]);
    }

    #[test]
    fn labels_and_separator_are_read() {
        let dir = tempdir("labels");
        let path = write(
            &dir,
            "config.toml",
            "modules = [\"cpu\"]\n\n[layout]\nseparator = \" :: \"\n\n[labels]\ncpu = \"Processor\"\n",
        );
        let config = resolve_with(
            &Cli {
                config: Some(path),
                ..cli()
            },
            None,
        )
        .unwrap();
        assert_eq!(config.separator, " :: ");
        assert_eq!(config.labels, vec![(ModuleId::Cpu, "Processor".to_owned())]);
    }

    #[test]
    fn no_file_configured_means_defaults_not_an_error() {
        // A path that does not exist is only an error when it was asked for
        // explicitly.
        let config = resolve_with(&cli(), Some(Path::new("/nonexistent/justfetch.toml"))).unwrap();
        assert_eq!(config.modules, ModuleId::ALL.to_vec());
    }

    #[test]
    fn every_malformed_file_reports_what_and_where() {
        let dir = tempdir("errors");
        for (name, body, expected) in [
            (
                "unknown-module.toml",
                "modules = [\"gpu\"]\n",
                "not a module id",
            ),
            (
                "duplicate.toml",
                "modules = [\"os\", \"os\"]\n",
                "listed twice",
            ),
            ("empty.toml", "modules = []\n", "empty"),
            (
                "bad-logo.toml",
                "[layout]\nlogo = \"big\"\n",
                "expected `auto` or `none`",
            ),
            (
                "empty-separator.toml",
                "[layout]\nseparator = \"\"\n",
                "separator is empty",
            ),
            (
                "bad-label.toml",
                "[labels]\ngpu = \"x\"\n",
                "not a module id",
            ),
            ("empty-label.toml", "[labels]\ncpu = \"  \"\n", "is empty"),
            ("unknown-key.toml", "moduls = [\"os\"]\n", "unknown field"),
            ("wrong-type.toml", "modules = \"os\"\n", "invalid type"),
            ("broken.toml", "modules = [\"os\"\n", "TOML parse error"),
        ] {
            let path = write(&dir, name, body);
            let error = resolve_with(
                &Cli {
                    config: Some(path.clone()),
                    ..cli()
                },
                None,
            )
            .expect_err(name);
            let shown = error.to_string();
            assert!(shown.contains(expected), "{name}: {shown}");
            assert!(
                shown.contains("config.toml") || shown.contains(name),
                "{name}: {shown}"
            );
        }
    }

    #[test]
    fn an_explicit_file_that_does_not_exist_is_an_error() {
        let error = resolve_with(
            &Cli {
                config: Some(PathBuf::from("/nonexistent/justfetch.toml")),
                ..cli()
            },
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("file not found"));
    }

    #[test]
    fn two_keys_that_name_one_module_are_rejected() {
        // `cpu` and `CPU` parse to the same module; accepting both would let
        // one label silently win.
        let dir = tempdir("duplicate-label");
        let path = write(&dir, "config.toml", "[labels]\ncpu = \"A\"\nCPU = \"B\"\n");
        let error = resolve_with(
            &Cli {
                config: Some(path),
                ..cli()
            },
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("same module"), "{error}");
    }

    #[test]
    fn the_module_argument_is_validated_the_same_way() {
        for (argument, expected) in [
            ("gpu", "not a module id"),
            ("os,os", "listed twice"),
            (" , ", "empty"),
        ] {
            let error = resolve_with(
                &Cli {
                    modules: Some(argument.to_owned()),
                    ..cli()
                },
                None,
            )
            .unwrap_err();
            assert!(error.to_string().contains(expected), "{argument}");
        }
    }
}
