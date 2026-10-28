use clap::Parser;
use justfetch::{
    cli::{Cli, Command, Format},
    commands, config, detect, privacy, render,
};
use std::{
    io::{self, Write},
    process::ExitCode,
};

/// Resolve the configuration once, reporting a failure the same way every
/// subcommand does.
fn resolved(cli: &Cli) -> Option<config::Config> {
    match config::resolve(cli) {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!("justfetch: {error}");
            None
        }
    }
}

fn run(cli: &Cli) -> io::Result<ExitCode> {
    match &cli.command {
        Some(Command::Init) => match commands::init(cli) {
            Ok(path) => {
                // Written through the same path as the main output, so a closed
                // stdout is a quiet success rather than a panic.
                let mut out = io::stdout().lock();
                writeln!(
                    out,
                    "wrote {}",
                    render::layout::sanitize(&path.display().to_string())
                )?;
                Ok(ExitCode::SUCCESS)
            }
            Err(error) => {
                eprintln!("justfetch: {error}");
                Ok(ExitCode::FAILURE)
            }
        },
        Some(Command::Doctor) => {
            let mut problems = 0;
            let mut out = io::stdout().lock();
            for finding in commands::diagnose(cli) {
                if finding.problem {
                    problems += 1;
                    writeln!(out, "problem: {}", finding.text)?;
                } else {
                    writeln!(out, "{}", finding.text)?;
                }
            }
            if problems == 0 {
                writeln!(out, "no problems found")?;
            }
            drop(out);
            Ok(if problems == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Some(Command::Themes) => {
            let Some(config) = resolved(cli) else {
                return Ok(ExitCode::FAILURE);
            };
            let listing = match cli.format {
                Format::Json => commands::themes_json(),
                Format::Text => {
                    let options = config.terminal_options();
                    commands::themes(options.depth, options.unicode, options.columns)
                }
            };
            io::stdout().lock().write_all(listing.as_bytes())?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let Some(config) = resolved(cli) else {
                return Ok(ExitCode::FAILURE);
            };
            // Only the selected modules are collected.
            let mut snapshot = detect::collect(&config.modules);
            if config.privacy {
                privacy::redact(&mut snapshot);
            }
            let output = match config.format {
                Format::Text => {
                    let logo = config.effective_logo(&snapshot);
                    render::terminal::render_with_logo(&snapshot, config.terminal_options(), &logo)
                }
                Format::Json => render::json::render(&snapshot).map_err(io::Error::other)?,
            };
            io::stdout().lock().write_all(output.as_bytes())?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("justfetch: {error}");
            ExitCode::FAILURE
        }
    }
}
