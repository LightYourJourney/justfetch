# Roadmap

This file is history plus what is still open. The current release state is in
[README](../README.md#status) and the per-platform evidence in
[compatibility](compatibility.md); the sections below record what each version
was for, not a promise that every item shipped in that order.

## v0.1 — Core (implementation candidate)

Implemented: single-package Rust CLI; eleven real detection fields; Linux/macOS/Windows adapters and conservative WSL detection; original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and fallback; width-aware terminal output, ANSI capability policy, no-color/no-logo, `--privacy`; tagged JSON with raw numeric units/source/confidence; independent field errors; deterministic parser/render tests and real CLI tests; three-OS CI; GPLv3-preserving documentation.

Release gate still external: exercise WSL, Intel macOS and distributions other than Ubuntu on real hardware (ARM64 Linux has had one non-interactive packaged-binary run in the release workflow, which is not an interactive check) (native CI has run and passed on Ubuntu 24.04, Windows Server 2025 and an ARM64 macOS runner, and has verified the 1.95 MSRV); take any published timing claim from a quiesced machine, since the instruction-count baseline is reproducible on a loaded host but wall clock is not (see benchmark.md); review package/name availability before publication. Evidence is in validation.md. v0.1.0 is published only as a pre-release, and nothing in it is a claim that every target device has been tested.

## v0.2 — Personalization

Implemented: a TOML behaviour config with documented per-platform paths; strict validation with unknown-key/module errors; the command line > `--config` > user file > defaults precedence; module selection, ordering, custom labels and a custom separator; `init` and `doctor`; and selection-driven collection rather than render-time hiding. See docs/configuration.md.

Also implemented: colour themes with a capability-aware reduction to 256 and 16 colours, four layouts, `justfetch themes`, custom theme files and custom logo files. See docs/themes.md.

Still open for this version: shareable sample configurations (point 6), which can follow once the schema has been used.

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



Add reliable GPU/battery/display/DE/WM/packages/locale collectors individually, each with documented sources, status semantics, fixtures and platform coverage. Disk and terminal are already collected; docs/validation.md records why GPU, display resolution, battery, packages and desktop/window manager were not added with them. Add measured per-module timing; `doctor` shipped with v0.2. Avoid shell-based package scans as a default shortcut.

## v0.4 — Polish

Broaden device/distro/WSL coverage, improve terminal heuristics and visual assets, measured performance refinement, compatibility/migration documentation. `--privacy` covers the identity fields already; broader redaction policy can grow with the later collectors.

## v1.0 — Stable

**State: v1.0.0-rc.1, release candidate.** Done: the CLI and JSON stability policy
(README), five-target release automation with packaged-binary smoke tests and a
counted `SHA256SUMS`, `cargo package` verification, the RustSec advisory check,
`SECURITY.md`, and draft Homebrew and Scoop manifests in `packaging/`.

Still open, and each one blocks turning the candidate into a release:

1. Execute the newer collectors on real Linux and Windows machines and record
   the output beside the native tools, as was done on macOS.
2. Exercise Intel macOS on real hardware; the candidate binary has only run
   under Rosetta 2.
3. Publish to crates.io, or state that cargo distribution stays
   `cargo install --path .` from a checkout. The name `justfetch` is absent from
   the crates.io index as of 2026-10-28, which is a point-in-time observation,
   not a reservation.
4. Fill the Homebrew and Scoop manifests with checksums from the real assets and
   submit them; nothing is submitted.
5. Remove `--draft --prerelease` from the release workflow — two words — after 1–4.
