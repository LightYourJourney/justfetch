# Changelog

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
- Record the raw 700-sample evidence in `docs/benchmarks/macos-arm64-interleaved.json`, including every metric Hyperfine emitted and the load samples, with no statistic baked in.
- Correct the hypothesis recorded in `docs/performance.md` for the 2026-10-11 re-measurement: the 3x wall-time drift was attributed to a per-run harness cost, and measuring sandboxed against unsandboxed showed it was host load instead.
- Report the result that collection and rendering are about 1.5% of the process's instructions — `--help` costs 27.4 M against 27.8 M for a full run — so the remaining cost is startup, linking and argument parsing.

### Release round

- Publish v0.1.0 as a GitHub pre-release with archives for five platforms, built by a new [release workflow](.github/workflows/release.yml). The pre-release flag is deliberate: the three platforms CI executes are not the whole matrix, and removing one word is the change once the real-hardware verification in the roadmap is done.
- Cross-compile the Intel macOS archive on an Apple Silicon runner rather than depend on the paid `-intel`/`-large` images, which are all that remain for x86_64 macOS now that macOS 13 is gone and 14 is deprecated. Verified locally to produce a working `x86_64` Mach-O binary.
- Write [docs/release-notes/v0.1.0.md](docs/release-notes/v0.1.0.md), which separates the platforms that were executed from the two that were only built, and states plainly that the archives are unsigned.
- Correct the README, which claimed no prebuilt release existed and that the project was unpublished.
- crates.io is deliberately untouched. Nothing in this round reserves the name there.

This version is published to GitHub as a pre-release, not to crates.io. Platform verification is recorded separately in docs/compatibility.md.
