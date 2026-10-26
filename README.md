# justfetch

**Just fetch your system. Beautifully.**

A compact Rust CLI that displays real local system information, with a responsive ASCII layout and structured JSON. It runs offline, requires no configuration or administrator access, and does not call other fetch tools.

**Status:** the published [v0.1.0 pre-release](https://github.com/LightYourJourney/justfetch/releases) carries the **seven-field** build described in its [release notes](docs/release-notes/v0.1.0.md). The eleven fields, `--privacy` and the wider test and CI work described below are in the repository and **not yet released**; nothing is on crates.io. Apple Silicon macOS was exercised locally, Linux x86-64 and Windows x64 in the CI test job, and ARM64 Linux in the release workflow, which also runs each packaged archive on its own runner except the cross-compiled Intel macOS one. Intel macOS is built but has never been run, so its archive is unverified; WSL and distributions other than Ubuntu remain untested. See the [compatibility matrix](docs/compatibility.md).

## Features

This list describes the current source. The published v0.1.0 archive is older and does not contain the eleven fields or `--privacy`; see Status above.

- OS, device model or labeled hostname, kernel, uptime, CPU, physical memory and swap, the filesystem holding your home directory, shell, terminal and local IPv4 addresses. Swap is reported as unsupported on Windows, which exposes no page-file figure to the APIs this project uses.
- Explicit available/unavailable/unsupported/redacted/error states, source and confidence in JSON.
- `--privacy` withholds the user name, the host name and the local addresses, and shortens the home path to `~`, identically in text and JSON.
- Original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and unknown systems.
- Unicode display-cell alignment and grapheme-aware wrapping; narrow terminals hide the logo.
- Conservative ANSI colors, NO_COLOR, TERM=dumb and redirected-output support; no terminal state changes.
- Control-sequence sanitization for displayed system/environment strings.
- No runtime network requests, telemetry, subprocesses or arbitrary configuration scripts.

## Example

Captured from a real `--no-color` invocation of the current source on Apple M3 / macOS 26.6.2; the v0.1.0 archive renders seven rows, not eleven. The user and host identity and the home path are replaced by shorter placeholders; the divider tracks the title's width, so it is shorter here than in the original. Memory, swap, disk and uptime naturally change. This is plain output; a compatible TTY adds colors automatically.

```text
         .'       ~ [user]@[host]
       .'         ───────────────
   .--.  .--.     OS      macOS 26.6.2
  /          \    Host    Mac15,12
 |            |   Kernel  Darwin 25.6.0
 |            |   Uptime  5h 54m
  \          /    CPU     Apple M3 (8 logical)
   `--.  .--'     Memory  13.47 / 16.00 GiB (84%)
                  Swap    11.84 / 13.00 GiB (91%)
                  Disk    302.06 / 460.43 GiB (66%) on /home/user
                  Shell   zsh
                  Term    iTerm.app 3.7.1beta1
                  IP      192.168.137.177 (en0)
                  ───────────────
                  ● ● ● ● ● ● ● ●
```

`~` marks estimated information, including the username environment hint. A shell fallback is displayed as `~ zsh (default)` so it cannot be mistaken for a confirmed current parent shell. Host hardware identifiers such as Mac15,12 are reported directly; no friendly-name guess is made.

## Build and install

Requires **stable Rust 1.95 or newer** and Cargo. From this source checkout:

```sh
cargo build --release --locked
./target/release/justfetch
cargo install --path . --locked
```

On Windows the built executable is `target\release\justfetch.exe`. Cargo installs into its configured bin directory; ensure that directory is on PATH. Prebuilt archives for five platforms are attached to the [v0.1.0 pre-release](https://github.com/LightYourJourney/justfetch/releases); they are unsigned, and no crates.io package, Homebrew formula or Scoop manifest exists for this version. No Rust nightly or special fonts are needed.

## Usage

```sh
justfetch
justfetch --help
justfetch --version
justfetch --no-color
justfetch --logo none
justfetch --privacy
justfetch --format text
justfetch --format json
```

`--privacy` is for output that leaves the machine. The user name, host name and local addresses are withheld: JSON marks those fields `"status": "redacted"` with no value, rather than inventing a placeholder string. The disk path is only shortened to `~` — that field stays `available`, because its counters are exact and `~` is a true shorthand for the directory that was read. A device-model Host row and everything else are left alone; shell and terminal are not redacted, because neither names a person.

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

The full document has `schema_version: 1`, identity/environment metadata and eleven field objects. Missing values have status/reason rather than fake N/A values; errors carry source/code/message. Consumers should ignore new additive fields. See [architecture](docs/architecture.md) and [accounting definitions](docs/compatibility.md).

## Platforms and accuracy

Linux reads os-release, proc and sysfs without assuming systemd or a package manager. macOS uses sysctl/Mach/uname and reports the exact CPU name. Windows uses native interfaces and a clearly labeled hostname rather than a slow WMI hardware query. WSL reports the Linux environment separately and never fabricates Windows host hardware; version is left unknown when evidence is insufficient.

Memory accounting differs across operating systems. Container values can reflect the host kernel rather than container quotas; cgroup accounting is outside v0.1. Shell detection checks the immediate parent executable, then a labeled configured default. Sandboxes may deny individual system APIs. [Compatibility](docs/compatibility.md) explains the boundaries and distinguishes implemented, compiled, tested and hardware-verified capabilities.

Usernames and hostnames can identify a person or device. Review output before sharing screenshots or JSON; `--privacy` withholds the identifying fields for anything that leaves the machine. justfetch does not upload it.

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Tests cover pure parsers, status/units, fixed rendering fixtures, long/wide/combining text, hostile terminal sequences, real CLI exits/streams and Unix pseudo-terminal color/width behavior. [CI](.github/workflows/ci.yml) runs format, lint, tests, a release build and a live JSON invocation on Ubuntu, macOS and Windows, plus a Rust 1.95 MSRV job; all four jobs pass. A passing runner is one configuration per platform, not a distribution matrix — see the [validation record](docs/validation.md).

The package has a small library for deterministic tests and a thin binary. Detection never draws, renderers never call system APIs, and every field keeps its provenance. Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before contributing.

- [Research, source evidence and dependency choices](docs/research.md)
- [Architecture and implementation plan](docs/architecture.md)
- [Validation record](docs/validation.md)
- [Measured performance and reproducible commands](docs/performance.md)
- [Version roadmap](docs/roadmap.md)
- [Changelog](CHANGELOG.md)

## License

[GNU GPL v3](LICENSE), declared `GPL-3.0-only`. The repository's existing license is preserved. No reference-project implementations or logo files were copied; bundled ASCII artwork uses the project license. Platform names do not imply vendor endorsement. Public name searches found other projects called justfetch; package-name checks are point-in-time, no crate name has been reserved, and nothing has been published to crates.io.
