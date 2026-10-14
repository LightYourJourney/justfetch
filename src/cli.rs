use clap::{Parser, ValueEnum};

/// Just fetch your system. Beautifully.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Disable ANSI colors.
    #[arg(long)]
    pub no_color: bool,
    /// Select the output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    /// Automatically select a built-in logo, or hide it.
    #[arg(long, value_enum, default_value_t = Logo::Auto)]
    pub logo: Logo,
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
