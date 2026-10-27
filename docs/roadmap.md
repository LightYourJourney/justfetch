# Roadmap

## v0.1 — Core (implementation candidate)

Implemented: single-package Rust CLI; eleven real detection fields; Linux/macOS/Windows adapters and conservative WSL detection; original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and fallback; width-aware terminal output, ANSI capability policy, no-color/no-logo, `--privacy`; tagged JSON with raw numeric units/source/confidence; independent field errors; deterministic parser/render tests and real CLI tests; three-OS CI; GPLv3-preserving documentation.

Release gate still external: exercise WSL, Intel macOS and distributions other than Ubuntu on real hardware (ARM64 Linux has had one non-interactive packaged-binary run in the release workflow, which is not an interactive check) (native CI has run and passed on Ubuntu 24.04, Windows Server 2025 and an ARM64 macOS runner, and has verified the 1.95 MSRV); take any published timing claim from a quiesced machine, since the instruction-count baseline is reproducible on a loaded host but wall clock is not (see performance.md); review package/name availability before publication. Evidence is in validation.md. v0.1.0 is published only as a pre-release, and nothing in it is a claim that every target device has been tested.

## v0.2 — Personalization

Implemented: a TOML behaviour config with documented per-platform paths; strict validation with unknown-key/module errors; the command line > `--config` > user file > defaults precedence; module selection, ordering, custom labels and a custom separator; `init` and `doctor`; and selection-driven collection rather than render-time hiding. See docs/configuration.md.

Still open for this version: visual themes (point 4) and custom logo files (point 5), which are Phase 11; shareable sample configurations (point 6) can follow once the schema has been used.

## v0.3 — Advanced Detection

Add reliable GPU/battery/display/DE/WM/packages/locale collectors individually, each with documented sources, status semantics, fixtures and platform coverage. Disk and terminal are already collected; docs/validation.md records why GPU, display resolution, battery, packages and desktop/window manager were not added with them. Add measured per-module timing; `doctor` shipped with v0.2. Avoid shell-based package scans as a default shortcut.

## v0.4 — Polish

Broaden device/distro/WSL coverage, improve terminal heuristics and visual assets, measured performance refinement, compatibility/migration documentation. `--privacy` covers the identity fields already; broader redaction policy can grow with the later collectors.

## v1.0 — Stable

Stable CLI/schema commitments, platform binaries and checksums, Cargo distribution, Homebrew/Scoop/winget packaging, automated verifiable releases and comprehensive security/license checks. These are not v0.1 implementation requirements.
