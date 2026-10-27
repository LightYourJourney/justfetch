use clap::Parser;
use justfetch::{
    cli::{Cli, Command, Format},
    commands, config, detect, privacy, render,
};
use std::{
    io::{self, Write},
    process::ExitCode,
};

fn run(cli: &Cli) -> io::Result<ExitCode> {
    match &cli.command {
        Some(Command::Init) => match commands::init(cli) {
            Ok(path) => {
                println!(
                    "wrote {}",
                    render::layout::sanitize(&path.display().to_string())
                );
                Ok(ExitCode::SUCCESS)
            }
            Err(error) => {
                eprintln!("justfetch: {error}");
                Ok(ExitCode::FAILURE)
            }
        },
        Some(Command::Doctor) => {
            let mut problems = 0;
            for finding in commands::diagnose(cli) {
                if finding.problem {
                    problems += 1;
                    println!("problem: {}", finding.text);
                } else {
                    println!("{}", finding.text);
                }
            }
            if problems == 0 {
                println!("no problems found");
            }
            Ok(if problems == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Some(Command::Themes) => {
            let config = match config::resolve(cli) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("justfetch: {error}");
                    return Ok(ExitCode::FAILURE);
                }
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
            let config = match config::resolve(cli) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("justfetch: {error}");
                    return Ok(ExitCode::FAILURE);
                }
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
