# Roadmap

## v0.1 — Core (implementation candidate)

Implemented: single-package Rust CLI; eleven real detection fields; Linux/macOS/Windows adapters and conservative WSL detection; original ASCII logos for macOS, Windows, Linux, Arch, Debian, Ubuntu, Fedora, Gentoo, openSUSE, NixOS and fallback; width-aware terminal output, ANSI capability policy, no-color/no-logo; tagged JSON with raw numeric units/source/confidence; independent field errors; deterministic parser/render tests and real CLI tests; three-OS CI; GPLv3-preserving documentation.

Release gate still external: exercise WSL, Intel macOS, ARM64 Linux and distributions other than Ubuntu on real hardware (native CI has run and passed on Ubuntu 24.04, Windows Server 2025 and an ARM64 macOS runner, and has verified the 1.95 MSRV); take any published timing claim from a quiesced machine, since the instruction-count baseline is reproducible on a loaded host but wall clock is not (see performance.md); review package/name availability before publication. Evidence is in validation.md. v0.1 is not a published release or a claim that every target device has been tested.

## v0.2 — Personalization

1. Define a versioned TOML behavior config and documented standard-directory policy (XDG on Linux, explicit macOS strategy, Windows user config). Preserve defaults without files.
2. Implement strict validation, helpful unknown-field/theme/module errors, and CLI > explicit file > user file > defaults precedence.
3. Add module ordering/selection without changing collectors or JSON field meanings.
4. Introduce complete style/theme definitions (Default, Catppuccin, Nord, Tokyo Night, Gruvbox), separate from behavior configuration.
5. Add bounded/sanitized custom-logo files and explicit logo choices; no config scripts.
6. Build init/config diagnostics and shareable samples once the schema is stable.

## v0.3 — Advanced Detection

Add reliable GPU/disk/battery/display/terminal/DE/WM/packages/locale collectors individually, each with documented sources, status semantics, fixtures and platform coverage. Add doctor and measured per-module timing. Avoid shell-based package scans as a default shortcut.

## v0.4 — Polish

Broaden device/distro/WSL coverage, improve terminal heuristics and visual assets, privacy mode, measured performance refinement, compatibility/migration documentation.

## v1.0 — Stable

Stable CLI/schema commitments, platform binaries and checksums, Cargo distribution, Homebrew/Scoop/winget packaging, automated verifiable releases and comprehensive security/license checks. These are not v0.1 implementation requirements.
