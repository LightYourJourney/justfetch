use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Just fetch your system. Beautifully.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Disable ANSI colors, overriding the configuration.
    #[arg(long, conflicts_with = "color")]
    pub no_color: bool,
    /// Force ANSI colors, overriding a configuration that disabled them.
    #[arg(long)]
    pub color: bool,
    /// Select the output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    /// Show only these modules, in this order (comma separated).
    ///
    /// Modules: os, host, kernel, uptime, cpu, memory, swap, disk, shell,
    /// terminal, local_ip.
    #[arg(long, value_name = "LIST")]
    pub modules: Option<String>,
    /// Read this configuration file instead of the user default.
    ///
    /// Accepted after a subcommand too, so `justfetch init --config PATH`
    /// works as well as `justfetch --config PATH init`.
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,
    /// Automatically select a built-in logo, or hide it. Defaults to auto.
    #[arg(long, value_enum)]
    pub logo: Option<Logo>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Logo {
    Auto,
    None,
}
