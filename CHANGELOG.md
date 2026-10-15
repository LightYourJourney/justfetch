# Changelog

## 0.1.0 — Unreleased

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

This version has not been published. Platform verification is recorded separately in docs/compatibility.md.
