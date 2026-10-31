# Roadmap

This file is history plus what is still open. The current release state is in
[README](../README.md#status) and the per-platform evidence in
[compatibility](compatibility.md); the sections below record what each version
was for, not a promise that every item shipped in that order.

## v0.1 — Core (implementation candidate)

Implemented: single-package Rust CLI; seven real detection fields; Linux/macOS/Windows adapters and conservative WSL detection; original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and fallback; width-aware terminal output, ANSI capability policy, no-color/no-logo; tagged JSON with raw numeric units/source/confidence; independent field errors; deterministic parser/render tests and real CLI tests; three-OS CI; GPLv3-preserving documentation.

Release gate still external: exercise WSL, Intel macOS and distributions other than Ubuntu on real hardware (ARM64 Linux has had one non-interactive packaged-binary run in the release workflow, which is not an interactive check) (native CI has run and passed on Ubuntu 24.04, Windows Server 2025 and an ARM64 macOS runner, and has verified the 1.95 MSRV); take any published timing claim from a quiesced machine, since the instruction-count baseline is reproducible on a loaded host but wall clock is not (see benchmark.md); review package/name availability before publication. Evidence is in validation.md. v0.1.0 is published only as a pre-release, and nothing in it is a claim that every target device has been tested.

## v0.2 — Personalization

Implemented: a TOML behaviour config with documented per-platform paths; strict validation with unknown-key/module errors; the command line > `--config` > user file > defaults precedence; module selection, ordering, custom labels and a custom separator; `init` and `doctor`; and selection-driven collection rather than render-time hiding. See docs/configuration.md.

Also implemented: colour themes with a capability-aware reduction to 256 and 16 colours, four layouts, `justfetch themes`, custom theme files and custom logo files. See docs/themes.md.

Sample configurations (point 6) followed later and now ship in `docs/examples/`.

## v0.3 — Advanced Detection

Phase 12 delivered GPU, Battery, Display, Packages, Desktop and Locale. Release
preparation still needs, in this order:

1. Run the Linux collectors on a real machine: dpkg and pacman counts, sysfs
   battery and display, desktop and session detection.
2. Run the Windows collectors on a real machine: `GetSystemPowerStatus` (the 128
   and 255 branches), `EnumDisplayDevicesW` vendor parsing, and
   `GetUserDefaultLocaleName`.
3. Exercise the multi-device paths on hardware this project has not had: two
   GPUs, two monitors, two battery packs.
4. Check WSL1 and WSL2 report `unsupported` for battery and display rather than
   inventing a value.
5. Write the JSON schema documentation for the new fields; the README only
   summarises them.
6. Decide and state whether the new additive fields keep `schema_version: 1`.



Add reliable GPU/battery/display/DE/WM/packages/locale collectors individually, each with documented sources, status semantics, fixtures and platform coverage. Shipped in Phase 12; `docs/compatibility.md` records the per-platform evidence and `docs/validation.md` the measurements. Measured per-module timing is in `docs/benchmark.md`, and the JSON contract is `docs/json-schema.json`. Avoid shell-based package scans as a default shortcut.

## v0.4 — Polish

Broaden device/distro/WSL coverage, improve terminal heuristics and visual assets, measured performance refinement, compatibility/migration documentation. `--privacy` covers the identity fields already; broader redaction policy can grow with the later collectors.

## v1.0 — Stable

**State: v1.0.0-rc.1, release candidate.** Done: the CLI and JSON stability policy
(README), five-target release automation with packaged-binary smoke tests and a
counted `SHA256SUMS`, `cargo package` verification, the RustSec advisory check,
`SECURITY.md`, and draft Homebrew and Scoop manifests in `packaging/`.

Two gates, not one. Unavailable hardware is a reason to state a limit, not a
reason to wait forever.

### Gate A — publish the release candidate for public trial

1. CI green on the exact commit that will be tagged: the three native runners,
   the MSRV job, the packaging job and the independent JSON Schema job.
2. No known P0 or P1 defect.
3. Release notes and compatibility matrix state what has and has not been
   executed, without overstating either.
4. The tag creates a **draft** prerelease; a maintainer publishes it.

The order of operations, and the commands, are in
[release-checklist](release-checklist.md). All four were met on 2026-10-30: `6dd54e9` passed all six CI jobs (run
40911728414), the tag was pushed, and its draft prerelease was reviewed and
published.

### Gate B — declare v1.0 stable

1. The release candidate has been available for public trial, and any defect it
   surfaced is closed.
2. At least one person has run `scripts/verify-linux.sh` on a real Linux machine
   and one `scripts/verify-windows.ps1` on real Windows, pasting the verdict
   table into `docs/validation.md`. The scripts exist so this does not depend on
   the maintainer owning that hardware.
3. Intel macOS is either exercised on hardware or its absence is accepted in
   writing, with the Rosetta evidence named as the substitute.
4. A decision on crates.io: publish, or state that installation is
   `cargo install --path .` from a checkout.
5. The prerelease marker follows the version: tagging `v1.0.0` publishes as
   stable without anyone editing the workflow, and `--draft` still holds
   publication for a maintainer. This one is done.

Until Gate B is met, the project stays a release candidate. Calling it stable
without those steps would be a claim the evidence does not support.
