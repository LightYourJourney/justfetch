# justfetch

**Just fetch your system. Beautifully.**

A compact Rust CLI that displays real local system information, with a responsive ASCII layout and structured JSON. It runs offline, requires no configuration or administrator access, and does not call other fetch tools.

**Status:** this source is **v1.0.0-rc.1**, a release candidate prepared for GitHub only. No `v1.0.0-rc.1` tag or release exists yet; the steps and the remaining gates are in [docs/roadmap.md](docs/roadmap.md). Nothing is on crates.io, and no Homebrew, Scoop or winget package is published. The previous release, v0.1.0, was a seven-field pre-release.

Verified by execution: macOS on Apple Silicon. The Intel macOS binary cross-compiles and runs under Rosetta 2, which is execution but not Intel hardware. Linux and Windows are built and tested natively in CI, and each runner smoke-tests its own packaged binary, so the collectors do execute there — but nothing compares their output with those platforms' own tools, and WSL and distributions other than Ubuntu are untested. See the [compatibility matrix](docs/compatibility.md) for the per-module evidence.

## Features

This list describes the current source. The published v0.1.0 archive is older and does not contain the seventeen fields or `--privacy`; see Status above.

- OS, device model or labeled hostname, kernel, uptime, CPU, graphics adapters, physical memory and swap, the filesystem holding your home directory, battery state, displays (opt-in), per-manager installed package counts, the desktop session (Linux), the process and system locale, shell, terminal and local IPv4 addresses. Swap is reported as unsupported on Windows, which exposes no page-file figure to the APIs this project uses.
- Explicit available/unavailable/unsupported/redacted/deselected/error states, source and confidence in JSON.
- `--privacy` withholds the user name, the host name and the local addresses, and shortens the home path to `~`, identically in text and JSON.
- Module selection and ordering, custom labels, a custom separator, and a TOML file to keep them in. Unselected modules are not collected at all. `justfetch init` writes a commented file; `justfetch doctor` explains what is wrong with a broken one.
- Six colour themes, four layouts and `justfetch themes` for discovery. A theme is a table of colours, so it cannot change what is measured or how it is arranged. True colour is reduced to 256 or 16 colours when the terminal cannot show it, and to plain text when colour is off.
- Built-in artwork follows the detected distribution, can be pinned by name, or replaced with a custom ASCII/Unicode file. Custom files are size-limited and stripped of escape sequences.
- Original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and unknown systems.
- Unicode display-cell alignment and grapheme-aware wrapping. Narrow terminals hide automatic artwork and keep the information; artwork you named yourself is stacked above it instead, unless the artwork is wider than the terminal.
- Conservative ANSI colors, NO_COLOR, TERM=dumb and redirected-output support; no terminal state changes.
- Control-sequence sanitization for displayed system/environment strings.
- No runtime network requests, telemetry, subprocesses or arbitrary configuration scripts.

## Example

Captured from a real `--no-color` invocation of the current source on Apple M3 / macOS 26.6.2; the v0.1.0 archive renders seven rows, not the fifteen a default run shows on macOS. `display` is not in the default list on any platform, and `desktop` is in it only on Linux, where the concept exists. The user and host identity and the home path are replaced by shorter placeholders; the divider tracks the title's width, so it is shorter here than in the original. Memory, swap, disk and uptime naturally change. This is plain output; a compatible TTY adds colors automatically.

```text
         .'       ~ [user]@[host]
       .'         ──────────────────────────────
   .--.  .--.     OS        macOS 26.6.2
  /          \    Host      Mac15,12
 |            |   Kernel    Darwin 25.6.0
 |            |   Uptime    6d 4h 25m
  \          /    CPU       Apple M3 (8 logical)
   `--.  .--'     GPU       Apple M3
                  Memory    12.67 / 16.00 GiB (79%)
                  Swap      5.72 / 7.00 GiB (82%)
                  Disk      345.36 / 460.43 GiB (75%) on /home/user
                  Battery   100% (full)
                  Packages  homebrew formulae 883, homebrew casks 44
                  Locale    zh_CN.UTF-8 (LANG), system zh_CN
                  Shell     zsh
                  Term      iTerm.app 3.7.1beta1
                  IP        192.168.137.185 (en0)
                  ──────────────────────────────
                  ● ● ● ● ● ● ● ●
```

`~` marks estimated information, including the username environment hint. A shell fallback is displayed as `~ zsh (default)` so it cannot be mistaken for a confirmed current parent shell. Host hardware identifiers such as Mac15,12 are reported directly; no friendly-name guess is made.

## Build and install

Requires **stable Rust 1.95 or newer** and Cargo. From this source checkout:

```sh
cargo install --path . --locked
```

Or take a prebuilt archive from the release: each holds the binary, `LICENSE` and `README.md`, and `SHA256SUMS` covers every archive. Verify it before running:

```sh
shasum -a 256 -c SHA256SUMS   # macOS; sha256sum on Linux
```

On Windows the built executable is `justfetch.exe`. Cargo installs into its configured bin directory; ensure that directory is on PATH. The archives are unsigned; macOS may require allowing the binary in System Settings after download. No Rust nightly, special fonts or administrator access are needed.

Draft Homebrew and Scoop manifests live in `packaging/` and are **not published**; their checksums must be filled from the real release assets first.

## Usage

```sh
justfetch
justfetch --help
justfetch --version
justfetch --no-color
justfetch --logo none
justfetch --logo arch
justfetch --logo-file ./art.txt
justfetch --theme nord
justfetch --theme-file ./theme.toml
justfetch --layout compact
justfetch themes
justfetch --privacy
justfetch --modules os,cpu,memory
justfetch --config ./justfetch.toml
justfetch --format text
justfetch --format json
justfetch init
justfetch doctor
```

`--privacy` is for output that leaves the machine. The user name, host name and local addresses are withheld: JSON marks those fields `"status": "redacted"` with no value, rather than inventing a placeholder string. The disk path is only shortened to `~` — that field stays `available`, because its counters are exact and `~` is a true shorthand for the directory that was read. A device-model Host row and everything else are left alone; shell and terminal are not redacted, because neither names a person.

`--theme` and `--layout` choose the look; `justfetch themes` lists the built-in
themes with a colour preview. `--logo auto` is the default; `--logo NAME` pins a
built-in emblem and `--logo-file PATH` draws your own. See [themes and
layouts](docs/themes.md).

`--modules` picks the rows and their order, and a module that is not selected is never collected — not merely hidden. Non-TTY output is plain text; nonempty NO_COLOR or TERM=dumb disables ANSI even in a TTY. Long fields wrap within the detected terminal width. Legacy non-UTF-8 locales use ASCII presentation; JSON retains the original strings.

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

The full document has `schema_version: 1`, identity/environment metadata and seventeen field objects. The shape is described formally in [json-schema.json](docs/json-schema.json), and a test validates every run against it. Missing values have status/reason rather than fake N/A values; errors carry source/code/message. Consumers should ignore new additive fields. See [architecture](docs/architecture.md) and [accounting definitions](docs/compatibility.md).

## Stability for 1.x

What 1.x promises, and what it does not:

- **JSON is the interface.** Existing keys keep their meaning and their units; new fields may be added, and a module that is not selected reports `status: "deselected"` rather than being omitted. `schema_version` stays `1` for additive changes and is only raised for a breaking one.
- **The command line is stable.** Flags, their values and the meaning of `--no-color`, `--privacy`, `--format` and the precedence order will not change within 1.x. New flags may appear.
- **Exit codes:** `0` for a successful run even when a field is unavailable, non-zero for invalid arguments, a broken configuration or a failed write.
- **The configuration file is strict.** An unknown key is an error rather than being ignored, so a file written for a newer version can fail on an older one. Additions within 1.x only add keys; removing or renaming one would be a breaking change.
- **Human-readable text is not an interface.** Columns, spacing, colours and the exact wording are presentation and may change in any release. Parse the JSON instead.

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

Tests cover pure parsers, status/units, fixed rendering fixtures, long/wide/combining text, hostile terminal sequences, real CLI exits/streams and Unix pseudo-terminal color/width behavior. [CI](.github/workflows/ci.yml) runs format, lint, tests, a release build and a live JSON invocation on Ubuntu, macOS and Windows, plus a Rust 1.95 MSRV job and a packaging job; all five pass. A passing runner is one configuration per platform, not a distribution matrix — see the [validation record](docs/validation.md).

The package has a small library for deterministic tests and a thin binary. Detection never draws, renderers never call system APIs, and every field keeps its provenance. Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before contributing.

- [Research, source evidence and dependency choices](docs/research.md)
- [Development guide](docs/development.md)
- [简体中文摘要](README.zh-CN.md)
- [Security policy](SECURITY.md)
- [Architecture and implementation plan](docs/architecture.md)
- [Validation record](docs/validation.md)
- [Measured performance and reproducible commands](docs/benchmark.md)
- [Configuration reference](docs/configuration.md)
- [Themes, layouts and logos](docs/themes.md)
- [Version roadmap](docs/roadmap.md)
- [Changelog](CHANGELOG.md)

## License

[GNU GPL v3](LICENSE), declared `GPL-3.0-only`. The repository's existing license is preserved. No reference-project implementations or logo files were copied; bundled ASCII artwork uses the project license. Platform names do not imply vendor endorsement. Public name searches found other projects called justfetch; package-name checks are point-in-time, no crate name has been reserved, and nothing has been published to crates.io.
