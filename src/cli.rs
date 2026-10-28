use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Just fetch your system. Beautifully.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Disable ANSI colors, overriding the configuration.
    #[arg(long, conflicts_with = "color", global = true)]
    pub no_color: bool,
    /// Force ANSI colors, overriding a configuration that disabled them.
    #[arg(long, global = true)]
    pub color: bool,
    /// Select the output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    /// Show only these modules, in this order (comma separated).
    ///
    /// Modules: os, host, kernel, uptime, cpu, gpu, memory, swap, disk,
    /// battery, shell, terminal, local_ip.
    #[arg(long, value_name = "LIST")]
    pub modules: Option<String>,
    /// Read this configuration file, layered over the user default.
    ///
    /// Values in this file override the user one; anything it omits keeps the
    /// user value. Accepted after a subcommand too, so `justfetch init
    /// --config PATH` works as well as `justfetch --config PATH init`.
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,
    /// Colour theme.
    ///
    /// Themes: default, catppuccin-mocha, nord, tokyo-night, gruvbox,
    /// monochrome. `justfetch themes` previews them.
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,
    /// Read a custom theme from this file instead of a built-in one.
    #[arg(long, value_name = "PATH")]
    pub theme_file: Option<PathBuf>,
    /// Arrange the output.
    ///
    /// Layouts: classic, compact, minimal, vertical.
    #[arg(long, value_name = "NAME")]
    pub layout: Option<String>,
    /// Draw a built-in logo instead of the detected one, or `none` to hide it.
    ///
    /// Logos: auto, none, macos, windows, linux, arch, debian, ubuntu, fedora,
    /// gentoo, opensuse, nixos, fallback.
    #[arg(long, value_name = "NAME")]
    pub logo: Option<String>,
    /// Draw a custom ASCII or Unicode logo from this file.
    #[arg(long, value_name = "PATH")]
    pub logo_file: Option<PathBuf>,
    /// Withhold the user name, host name and local IP addresses, and shorten
    /// the home path to ~.
    #[arg(long)]
    pub privacy: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write a commented default configuration file.
    Init,
    /// Check the configuration and report what is wrong with it.
    Doctor,
    /// List the built-in themes with a preview of each.
    Themes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}
