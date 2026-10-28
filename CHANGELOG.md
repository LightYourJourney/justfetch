# Changelog

## Unreleased

### More fields

- Add **Swap**, **Disk**, **Terminal** and **Local IP**, taking the field set from seven to eleven. These are the four fields every major fetch tool shows that could be collected on all three platforms without a subprocess, administrator access, a directory scan or a private-file read. Each keeps the existing provenance contract: a source string, a confidence level and a stated reason when unavailable.
- Disk reports the filesystem containing the home directory, via `statvfs` on Unix and `GetDiskFreeSpaceExW` on Windows. `used` is `total - free` and `available` is what a user may still allocate, so the two coincide where no blocks are reserved and differ by the reserve where they are. On APFS the kernel reports container-wide counters, so the figure matches Fastfetch's and can differ from `df`'s per-volume usage.
- Local IP lists every routable IPv4 address rather than picking one, because the first interface the kernel reports is often a container bridge. Windows walks `GetAdaptersAddresses` and filters on `OperStatus`.
- Swap is exact on Linux and macOS. Windows reports `unsupported`, for the reason measured below rather than assumed.
- Terminal prefers the emulator name over the shell: `TERM_PROGRAM` (with its version when set), then `WT_SESSION`, then `TERM`. It reads environment variables only, so redirected output reports the same terminal. How the `TERM` fallback is classified is described under Stabilisation below.
- `f_blocks` is a 32-bit counter on macOS and a 64-bit one on Linux, so the widening conversion is explicit on both instead of relying on a cast that happens to compile.
- Local IP filters loopback, link-local and unspecified addresses through one shared predicate, so the Unix and Windows adapters exclude the same set. An address on an adapter the platform cannot name is reported without an empty `()`, and the interface field is omitted from JSON rather than sent as `""`.
- Windows swap now reports `unsupported`, the first field to use that state. Windows exposes a commit limit rather than page-file usage, and the derivation from it does not hold: the CI runner reported 2.41 GiB committed against 2.62 GiB resident, so the subtraction went negative, because file-backed mappings and driver memory are resident without being committed. No permitted API exposes the figure, so it is named as unsupported rather than approximated.

### Stabilisation

- Terminal detection no longer presents `TERM` as the terminal application. `TERM_PROGRAM` and `WT_SESSION` name the program and stay confirmed; `TERM` carries a capability type such as `xterm-256color`, so it is reported with `"kind": "capability_type"` and marked estimated with a note. The text row reads `~ xterm-256color (TERM type)`.
- Add `--privacy`, which withholds the user name, host name and local addresses and shortens the home path to `~`, identically in text and JSON. Withheld fields report a new `redacted` status rather than a placeholder string, so a withheld value cannot be mistaken for an invented one.
- Replace the renderer's parallel field arrays with one ordered row list, and derive the label column width from the labels instead of a constant. Pinned output is unchanged; an independent review then found that the narrow branch (8–10 columns) had lost the label accent, which is now restored and covered by a width sweep. The list is the one place a later version would select, reorder or relabel fields.
- Add `tests/detection.rs`, which asserts on every platform that the core fields are actually available and that the reported numbers are self-consistent. The CI smoke step previously proved only that the process started.
- The release workflow now unpacks the archive it just produced and runs the binary from inside it, for every target that can run on its own runner, and checks that `SHA256SUMS` lists every archive. The cross-compiled Intel macOS archive is excluded explicitly and stays built-but-not-executed.
- Both workflows assert named fields (`os` and `memory`) rather than "some field is available", which one working detector could satisfy. Tests were added for a closed stdout and for ANSI support following the platform console.
- Windows rejects a `SOCKADDR` whose declared length is shorter than the structure it is cast to.
- Re-measure the performance baseline on the eleven-field binary, asking each competitor for as many of the same fields as it supports. Four extra fields cost 1.4% more instructions; the competitor figures from the previous round are no longer comparable and are labelled as such.

### Modules and configuration

- Add `ModuleId`, one stable identifier per module. Selection, ordering, labels, diagnostics, collection and rendering all key off it, so a module is named once instead of appearing in several parallel lists.
- Make collection follow the selection instead of filtering at render time. An unselected module is never queried: `--modules cpu` skips the address enumeration, the filesystem query, the process lookup and the registry reads the other rows would need. Measured saving is about 2% of the process's instructions for a one-module selection, which is real but dwarfed by process startup; the numbers and raw data are in `docs/validation.md`.
- Add a TOML configuration file covering `modules`, `layout.logo`, `layout.separator`, `color.enabled`, `color.accent` and per-module `labels`. Unknown keys, unknown modules, duplicate modules, empty lists, blank labels and wrong value types are errors with the file and position that produced them. A missing file is not an error.
- Add the precedence chain — command line, `--config`, user file, defaults — in one function, with tests for each layer and for a conflicting pair. `--config` layers over the user file rather than replacing it, so an explicit file can set one key and inherit the rest.
- Add `--modules`, `--config` and `--color`; `--no-color` keeps its meaning and now conflicts with `--color` rather than being ignored.
- Add `justfetch init`, which writes a commented file with every key at its default and refuses to overwrite an existing one, and `justfetch doctor`, which reports the resolved path, whether it exists, the effective settings and any configuration error, exiting non-zero when it finds one.
- Add a `deselected` field status so an unselected module is distinguishable from one that failed, was unavailable or is unsupported. Default runs are unchanged; `schema_version` stays 1.
- Document the whole system in `docs/configuration.md`.
- 87 tests, up from 54.

### Themes, layouts and logos

- Add a theme model that is a table of colour roles — title, label, value, divider, emphasis, logo, logo secondary and swatches — and nothing else. It contains no collection or formatting code, so a theme cannot change what is measured or how it is arranged.
- Ship six themes: `default` (the terminal's own sixteen colours, artwork in its distribution colour), `catppuccin-mocha`, `nord`, `tokyo-night`, `gruvbox` and `monochrome`. `justfetch themes` previews each one — its name in the theme's own title colour, its palette, and a label/value/rule sample — and with `--format json` it emits the same list as a JSON document.
- Reduce colour to what the terminal can show: exact RGB when `COLORTERM` advertises true colour, the nearest xterm-256 index for a 256-colour terminal, the nearest of the sixteen basic colours otherwise, and no escapes at all when colour is off. The default theme asks only for named colours, so it never depends on an approximation.
- Add four layouts: `classic` (artwork beside the information), `compact` (aligned rows without artwork or decoration), `minimal` (`Label: value`, unaligned, for narrow output) and `vertical` (artwork centred above). Narrow terminals lose the rules and swatches first, then the artwork, and never the information.
- Add `--theme`, `--theme-file PATH`, `--layout`, `--logo NAME` (a built-in emblem) and `--logo-file PATH` (your own ASCII or Unicode artwork), each with the same precedence as the existing settings: command line, `--config` file, user file, defaults. A tighter layer's logo choice replaces inherited artwork rather than being silently overridden by it. Artwork chosen explicitly is not dropped for width while it fits the terminal: `classic` moves it above the information instead. Artwork wider than the terminal cannot be drawn at all.
- Add `[theme] name`/`[theme] file` and `[layout] name`/`logo_file` to the configuration file. A file named there is resolved relative to the configuration file that names it.
- Bound and clean user artwork: 32 KiB, 40 lines and 160 columns per line, escape sequences stripped, trailing spaces removed, empty refused. Custom theme files are limited to 16 KiB and validated key by key.
- The artwork is shaded between the theme's two logo colours when both are set and the terminal can express the intermediate shades; named colours are not blended into invented ones.
- JSON output is untouched by any of these settings. `schema_version` stays 1.
- 138 tests, up from 87.

## 0.1.0 — pre-release

- Add Rust CLI with text/JSON, no-color and automatic/no-logo options.
- Collect OS, Host, Kernel, Uptime, CPU, Memory and Shell through platform interfaces.
- Preserve detection status, provenance, confidence, raw bytes/seconds and field errors.
- Add Linux/macOS/Windows adapters and conservative WSL semantics.
- Add eleven original built-in ASCII logos, Unicode-aware responsive layout and terminal-control sanitization.
- Add parser, renderer and CLI tests; native three-platform CI and an MSRV check.
- Document compatibility, architecture, research, measured performance and release limitations.
- Preserve the repository's existing GPLv3 license.

### Corrections before the first release

- Fix the renderer emitting an empty styled span, an opening escape pair immediately followed by a reset, for every row below a logo that is shorter than the information column. Measured on the built-in macOS logo this removes 6 of 78 escape sequences per invocation; shorter logos remove more. The visible output is unchanged.
- Add a regression test that fails without that fix, plus two property tests sweeping 0–260, 1000, 4096 and 100000 columns across all color/Unicode combinations. That round took the test count from 27 to 31.
- Correct `docs/compatibility.md`: long fields wrap on grapheme boundaries but a grapheme wider than the columns left for it is replaced by `?` rather than preserved, and the logo threshold is stated as the actual rule instead of an unexplained one.
- Correct `docs/compatibility.md` on Host fallback: Linux falls back to the labeled hostname only when a model source is absent, while macOS never falls back and reports an error instead.
- Record an independent re-measurement in `docs/performance.md`. Peak RSS reproduced to within 0.05 MiB, but wall times and the Fastfetch/pfetch-rs ranking did not reproduce on the loaded measurement machine. The earlier wall times are now marked as an upper bound, not a baseline.

### Code review round

- Remove the last two panics from production paths: `src/platform/linux.rs` no longer routes `unreachable!` through `Detection::map`.
- Check the `mach_port_deallocate` result in the macOS memory detector instead of discarding it, so a send right that cannot be released is not reported as a clean read.
- Move the duplicated uname hostname/kernel construction into `platform::uname_identity`, which is now unit-tested for both the success and the failure path.
- Let `logo::select` choose the generic Linux artwork from the platform family instead of `terminal::render` comparing the logo's name string; add the test that branch never had.
- Give the seven field keys one definition instead of two, and tie the styled-prefix width to its format string by a test rather than a bare `8`.
- Replace `assets/logos/linux.txt`. It was the widely-circulated Tux ASCII art of unknown authorship while the adjacent notice claimed every file was authored for this project; the artwork is now an original drawing and the provenance notice is accurate. `docs/third-party.md` gained a bundled-asset section.
- Test count is now **35**.

### Benchmark round

- Measure Macchina 6.4.0, the comparator the spec asked for and the two earlier rounds had to leave open. It is included in the baseline table and in the new raw artifact.
- Move the baseline onto instruction counts. Wall clock on a loaded host varied by 59–125% across commands while instruction counts varied by 1.2–2.1%, so wall clock was demoted to a load-contaminated upper bound. Ten interleaved rounds with a rotating command order replaced the sequential protocol, which let a single load spike bias a whole command.
- Record the raw 700-sample evidence in `docs/benchmarks/macos-arm64-interleaved-2026-10-16.json`, including every metric Hyperfine emitted and the load samples, with no statistic baked in.
- Correct the hypothesis recorded in `docs/performance.md` for the 2026-10-11 re-measurement: the 3x wall-time drift was attributed to a per-run harness cost, and measuring sandboxed against unsandboxed showed it was host load instead.
- Report the result that collection and rendering are about 1.5% of the process's instructions — `--help` costs 27.4 M against 27.8 M for a full run — so the remaining cost is startup, linking and argument parsing.

### Release round

- Publish v0.1.0 as a GitHub pre-release with archives for five platforms, built by a new [release workflow](.github/workflows/release.yml). The pre-release flag is deliberate: the three platforms CI executes are not the whole matrix, and removing one word is the change once the real-hardware verification in the roadmap is done.
- Cross-compile the Intel macOS archive on an Apple Silicon runner rather than depend on the paid `-intel`/`-large` images, which are all that remain for x86_64 macOS now that macOS 13 is gone and 14 is deprecated. Verified locally to produce a working `x86_64` Mach-O binary.
- Write [docs/release-notes/v0.1.0.md](docs/release-notes/v0.1.0.md), which separates the platforms that were executed from the two that were only built, and states plainly that the archives are unsigned.
- Correct the README, which claimed no prebuilt release existed and that the project was unpublished.
- Fix a four-fold undercount of used memory that the Intel macOS archive carried, found by running the artifact this round had just produced. `src/platform/macos.rs` paired the page counters from `host_statistics64` with the page size reported by `sysconf(_SC_PAGESIZE)`. Those agree on real hardware, but under Rosetta the process is told 4096 while the counters are in the host's 16384-byte pages, so the cross-compiled archive reported 3.4 GiB where the native build reported 13.7 GiB. It now asks `host_page_size()` — the only one of the three candidate sources that returns the host value; `libc` exports `vm_page_size`, but measuring it showed it carries the same wrong 4096 under Rosetta. Verified against the native build to within 6 MiB across four alternating runs. No unit test covers this: the defect lives in which system API is trusted, not in arithmetic that a fixture can reach, and the check that did exist (`used <= total`) is passed by a four-fold undercount. On genuine Intel hardware both APIs return 4096, so the change is expected to be a no-op there.
- crates.io is deliberately untouched. Nothing in this round reserves the name there.

This version is published to GitHub as a pre-release, not to crates.io. Platform verification is recorded separately in docs/compatibility.md.
