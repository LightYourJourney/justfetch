use clap::Parser;
use justfetch::{
    cli::{Cli, Format},
    config::Config,
    detect, privacy, render,
};
use std::{
    io::{self, Write},
    process::ExitCode,
};

fn run() -> io::Result<()> {
    let config = Config::from(Cli::parse());
    let mut snapshot = detect::collect();
    if config.privacy {
        privacy::redact(&mut snapshot);
    }
    let output = match config.format {
        Format::Text => render::terminal::render(&snapshot, config.terminal_options()),
        Format::Json => render::json::render(&snapshot).map_err(io::Error::other)?,
    };
    io::stdout().lock().write_all(output.as_bytes())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("justfetch: {error}");
            ExitCode::FAILURE
        }
    }
}
