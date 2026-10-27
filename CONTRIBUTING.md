# Contributing

Read [AGENTS.md](AGENTS.md) for architecture, commands and accuracy rules. Rust 1.95+ is required. Run fmt, Clippy, tests and a release build before proposing changes. Keep changes scoped; include a regression fixture when fixing detection or rendering. New collectors must document their units, provenance, unavailable/error semantics and platform evidence. Never submit copied logos/code without license provenance. Report OS/architecture, toolchain, command and redacted text/JSON when reporting bugs. Avoid posting private hostnames or usernames unintentionally.

The v0.1 roadmap deliberately excluded configuration, themes and extra collectors; the TOML configuration file and module selection have since shipped, while visual themes and additional collectors remain out of scope until their phase. Discuss large scope changes before implementing them. Contributions are under the repository GPL-3.0-only license.
