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
- Add a regression test that fails without that fix, plus two property tests sweeping 0–260, 1000, 4096 and 100000 columns across all color/Unicode combinations. Test count is now 31.
- Correct `docs/compatibility.md`: long fields wrap on grapheme boundaries but a grapheme wider than the columns left for it is replaced by `?` rather than preserved, and the logo threshold is stated as the actual rule instead of an unexplained one.
- Correct `docs/compatibility.md` on Host fallback: Linux falls back to the labeled hostname only when a model source is absent, while macOS never falls back and reports an error instead.
- Record an independent re-measurement in `docs/performance.md`. Peak RSS reproduced to within 0.05 MiB, but wall times and the Fastfetch/pfetch-rs ranking did not reproduce on the loaded measurement machine. The earlier wall times are now marked as an upper bound, not a baseline.

This version has not been published. Platform verification is recorded separately in docs/compatibility.md.
