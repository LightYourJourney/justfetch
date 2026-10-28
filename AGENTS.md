# justfetch contributor and agent guidance

## Goal and scope

Build a fast, accurate, offline Rust system-information CLI. Current scope: OS, Host, Kernel, Uptime, CPU, GPU, Memory, Swap, Disk, Battery, Display, Packages, Shell, Terminal, Local IP, terminal/JSON, original ASCII logos and graceful field failures. Swap is unsupported on Windows, which exposes no page-file usage through the APIs this project permits. Module selection and ordering (`--modules` or a TOML file), custom labels, a custom separator, `init` and `doctor` are implemented; see docs/configuration.md. Colour themes, four layouts, custom logo files and the `themes` listing are implemented; see docs/themes.md. Arbitrary plugin modules and additional collectors belong to later releases; see docs/roadmap.md.

## Architecture

- src/main.rs: CLI lifecycle, stdout/stderr and exit codes.
- src/cli.rs, config.rs and commands.rs: arguments, the TOML file, theme and logo files, precedence, read-only terminal capabilities, and the `init`/`doctor`/`themes` subcommands.
- src/module.rs: the stable module identifiers that selection, ordering, labels and collection all key off.
- src/model.rs: serde schema, raw numeric units, source/confidence and distinct states.
- src/detect/: orchestration, pure Linux parsers and conservative shell recognition.
- src/platform/: all native OS access; no formatting or printing.
- src/render/: deterministic terminal/JSON rendering; no system calls or environment reads. `colour.rs` holds the truecolour/256/16 capability reduction, `theme.rs` the colour roles and built-in themes, `logo.rs` the artwork registry and validation, `layout.rs` the arrangements plus width and sanitisation helpers.
- assets/logos/: built-in ASCII, licensed with this project.
- tests/: real CLI execution and fixed-snapshot rendering.

Keep one package with lib/bin. Avoid a plugin registry or TUI framework. Do not create empty modules. Preserve existing LICENSE (GPL-3.0-only) and user files. Do not copy code/assets with unverified licenses. No commit, push, release or publication unless requested.

## Rust conventions and semantics

Stable Rust, edition 2024, MSRV 1.95. Use explicit bytes/seconds in models; never use Unknown/N/A strings to erase states. Preserve field-level errors, distinguish hostname from model and default from current shell. Prefer safe Rust; any necessary FFI must have a narrow wrapper, checked results, correct ownership and SAFETY comments. No unsafe micro-optimizations. Avoid unwrap/expect/panic in production paths. Do not silently clamp invalid memory counters into plausible results.

Do not add runtime network requests, telemetry, administrator requirements, external fetch backends, filesystem scans, private-file reads or subprocesses. The only files read outside system paths are the user's own configuration file and the theme or logo files a configuration names or `--theme-file`/`--logo-file` point at. They are read at those resolved paths and nowhere else, and nothing is written unless `init` is run. Refresh only required CPU fields. Unix shell lookup is direct; Windows uses one documented parent-ID snapshot and one parent-image query. Never execute a shell string. Treat all system and environment strings as untrusted terminal input; sanitize before width calculation and add styling afterwards. Renderers receive all capabilities explicitly; never change terminal state. Keep JSON stdout to exactly one JSON document, even with visual options.

## Commands and evidence

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
cargo run -- --no-color
cargo run -- --format json
```

Use pure parser fixtures and fixed SystemSnapshot fixtures for state/Unicode/layout regressions. Test real CLI help/version/invalid flags/exit codes/stdout/stderr. Reproduce defects with a regression test, fix the source, then rerun relevant checks. Keep failing tests; never lower correct acceptance standards to get green output. Cross-target cargo check proves compilation only, not linking or actual detection. CI workflow presence is not CI success. Report exact validation and platform limitations in docs/compatibility.md and docs/validation.md. Record benchmark environment/commands/raw output; never quote upstream README speeds as measured results.

## Known risks

WSL1 kernel/memory are environment reports; do not claim Windows host hardware. Linux old-kernel memory fallback is an estimate. macOS kern.boottime may be sandbox-restricted. Windows version strings and parent-process availability depend on native permissions/registry; do not guess on failure. Shell detection is immediate-parent only, then labeled configured default. Unicode width is terminal/font-dependent even with standard width tables. Name availability is not reserved until publication.

## Current library documentation

Use Context7 MCP whenever changing library/framework/SDK/API/CLI/cloud-specific usage. Resolve the library ID first unless the user gives an exact /org/project ID; query one concept per call, then implement using fetched docs and the pinned release source when main differs. This includes well-known libraries. Do not use it for general programming, business logic, code review or pure refactoring.

## CodeGraph

If a .codegraph directory exists at the repository root, use codegraph_explore MCP (or `codegraph explore "question"`) before grep/find or source reads when locating or understanding code. If absent, skip CodeGraph; do not index without the user's decision.
