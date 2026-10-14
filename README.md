# justfetch

**Just fetch your system. Beautifully.**

A compact Rust CLI that displays real local system information, with a responsive ASCII layout and structured JSON. It runs offline, requires no configuration or administrator access, and does not call other fetch tools.

**Status:** v0.1 implementation candidate, not yet published. Apple Silicon macOS has been exercised locally. Linux, Windows and Intel macOS adapters have cross-target compile checks; their native CI and hardware validation remain pending. See the [compatibility matrix](docs/compatibility.md).

## Features

- OS, device model or labeled hostname, kernel, uptime, CPU, physical memory and shell.
- Explicit available/unavailable/unsupported/error states, source and confidence in JSON.
- Original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and unknown systems.
- Unicode display-cell alignment and grapheme-aware wrapping; narrow terminals hide the logo.
- Conservative ANSI colors, NO_COLOR, TERM=dumb and redirected-output support; no terminal state changes.
- Control-sequence sanitization for displayed system/environment strings.
- No runtime network requests, telemetry, subprocesses or arbitrary configuration scripts.

## Example

Captured from a real invocation on Apple M3 / macOS 26.6.2; only the user and host identity below are redacted. Memory and uptime naturally change. This is plain output; a compatible TTY adds colors automatically.

```text
         .'       ~ [user]@[host]
       .'         ------------------------------
   .--.  .--.     OS      macOS 26.6.2
  /          \    Host    Mac15,12
 |            |   Kernel  Darwin 25.6.0
 |            |   Uptime  9d 14h 53m
  \          /    CPU     Apple M3 (8 logical)
   `--.  .--'     Memory  13.41 / 16.00 GiB (84%)
                  Shell   zsh
                  ------------------------------
                  o o o o o o o o
```

`~` marks estimated information, including the username environment hint. A shell fallback is displayed as `~ zsh (default)` so it cannot be mistaken for a confirmed current parent shell. Host hardware identifiers such as Mac15,12 are reported directly; no friendly-name guess is made.

## Build and install

Requires **stable Rust 1.95 or newer** and Cargo. From this source checkout:

```sh
cargo build --release --locked
./target/release/justfetch
cargo install --path . --locked
```

On Windows the built executable is `target\release\justfetch.exe`. Cargo installs into its configured bin directory; ensure that directory is on PATH. No crates.io package, Homebrew formula, Scoop manifest or prebuilt release is claimed available for this version. No Rust nightly or special fonts are needed.

## Usage

```sh
justfetch
justfetch --help
justfetch --version
justfetch --no-color
justfetch --logo none
justfetch --format text
justfetch --format json
```

`--logo auto` is the default. Other explicit logo choices and configurable themes/modules are planned, not implemented in v0.1. Non-TTY output is plain text; nonempty NO_COLOR or TERM=dumb disables ANSI even in a TTY. Long fields wrap within the detected terminal width. Legacy non-UTF-8 locales use ASCII presentation; JSON retains the original strings.

JSON stdout is exactly one document, with raw seconds/bytes and no logo, ANSI or diagnostic logs. Field failures remain structured data; successful output returns zero even when a field is unavailable. Invalid options return a nonzero code with diagnostics on stderr. Output-write failures also return nonzero; closed pipes exit quietly.

For example, an available memory field has this structure (numbers here illustrate the schema):

```json
{
  "status": "available",
  "value": {
    "total_bytes": 17179869184,
    "used_bytes": 8589934592,
    "definition": "total_minus_available",
    "scope": "operating_system"
  },
  "source": "/proc/meminfo",
  "confidence": "confirmed"
}
```

The full document has `schema_version: 1`, identity/environment metadata and seven field objects. Missing values have status/reason rather than fake N/A values; errors carry source/code/message. Consumers should ignore new additive fields. See [architecture](docs/architecture.md) and [accounting definitions](docs/compatibility.md).

## Platforms and accuracy

Linux reads os-release, proc and sysfs without assuming systemd or a package manager. macOS uses sysctl/Mach/uname and reports the exact CPU name. Windows uses native interfaces and a clearly labeled hostname rather than a slow WMI hardware query. WSL reports the Linux environment separately and never fabricates Windows host hardware; version is left unknown when evidence is insufficient.

Memory accounting differs across operating systems. Container values can reflect the host kernel rather than container quotas; cgroup accounting is outside v0.1. Shell detection checks the immediate parent executable, then a labeled configured default. Sandboxes may deny individual system APIs. [Compatibility](docs/compatibility.md) explains the boundaries and distinguishes implemented, compiled, tested and hardware-verified capabilities.

Usernames and hostnames can identify a person or device. Review/redact output before sharing screenshots or JSON. justfetch does not upload it; privacy mode is planned for a later version.

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Tests cover pure parsers, status/units, fixed rendering fixtures, long/wide/combining text, hostile terminal sequences, real CLI exits/streams and Unix pseudo-terminal color/width behavior. [CI](.github/workflows/ci.yml) is configured for Ubuntu, macOS and Windows, plus an MSRV job; configured CI is not a claim of completed remote runs.

The package has a small library for deterministic tests and a thin binary. Detection never draws, renderers never call system APIs, and every field keeps its provenance. Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before contributing.

- [Research, source evidence and dependency choices](docs/research.md)
- [Architecture and implementation plan](docs/architecture.md)
- [Validation record](docs/validation.md)
- [Measured performance and reproducible commands](docs/performance.md)
- [Version roadmap](docs/roadmap.md)
- [Changelog](CHANGELOG.md)

## License

[GNU GPL v3](LICENSE), declared `GPL-3.0-only`. The repository's existing license is preserved. No reference-project implementations or logo files were copied; bundled ASCII artwork uses the project license. Platform names do not imply vendor endorsement. Public name searches found other projects called justfetch; package-name checks are point-in-time and no name has been reserved or package published.
