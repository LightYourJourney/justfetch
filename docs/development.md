# Development

Everything here runs offline. The project has no runtime network access, and the
checks below need nothing but a Rust toolchain.

## Requirements

- Stable Rust **1.95 or newer** (the MSRV; CI checks it on every push).
- On Windows, the MSVC toolchain; nothing else.
- No build scripts beyond Cargo's own, no external code generators, no fonts.

## Everyday loop

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

`cargo run -- --no-color` and `cargo run -- --format json` exercise the two
output paths. Tests are expected to fail loudly: never loosen an assertion or
delete a test to get a green run.

## Cross-platform checks

Cross-compilation proves the code compiles for a target, not that it runs there.

```sh
# Compile for the other two operating systems (needs their std installed).
cargo check --locked --all-targets --target x86_64-pc-windows-gnu
cargo check --locked --all-targets --target x86_64-unknown-linux-gnu
cargo check --locked --all-targets --target aarch64-unknown-linux-gnu

# The MSRV.
rustup run 1.95.0 cargo check --locked --all-targets
```

Building a runnable binary needs a linker for the target: the MinGW toolchain
for `*-pc-windows-gnu`, a cross GCC for the Linux targets. Without one the link
step fails and the target stays compile-checked; CI builds each target on its
own native runner instead.

The Intel macOS binary cross-compiles on an Apple Silicon host and runs under
Rosetta 2, which is how it was exercised locally. That is execution, but not
Intel hardware.

## Where things live

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | CLI lifecycle, exit codes, stdout/stderr |
| `src/cli.rs` | argument parsing only |
| `src/config.rs` | TOML file, precedence, paths |
| `src/commands.rs` | `init`, `doctor`, `themes` |
| `src/module.rs` | the module identifiers, ordering and defaults |
| `src/model.rs` | the JSON schema and its states |
| `src/detect/` | orchestration and pure parsers (testable on any host) |
| `src/platform/` | native OS access; no formatting, no printing |
| `src/render/` | deterministic rendering; no system calls, no environment reads |
| `assets/logos/` | first-party ASCII artwork |
| `tests/` | real CLI runs and fixed-snapshot rendering |

The boundary that matters: `render/` never reads the environment and `platform/`
never prints. If a change needs to cross that line, the capability should be
passed in instead.

## Adding a module

1. `ModuleId` variant, identifier, label, and the position in `ALL`.
2. Decide whether it belongs in `DEFAULT`; a module that is expensive or
   meaningless on most platforms does not.
3. Model type in `model.rs`, with units in the field names and a state for
   "not applicable" that is distinct from zero.
4. Collection in each platform file, gated by `if_selected`.
5. A rendering arm; the compiler will demand it.
6. Tests: a value, an absent value, an unsupported platform, and the JSON shape.
7. Documentation: `docs/configuration.md`, `docs/compatibility.md`, README.

## Release process

Prepared, not automated:

1. Bump `version` in `Cargo.toml` and `Cargo.lock`, add a
   `docs/release-notes/v<version>.md`, and update `CHANGELOG.md`.
2. `cargo package --locked` must build from the packaged file list.
3. Push a `v*` tag. The workflow checks that the tag matches `Cargo.toml`,
   builds five targets, smoke-tests the packaged binary on every runner that can
   run it, and writes `SHA256SUMS` with a count check.
4. It then creates a **draft prerelease**. Publishing that draft is a maintainer
   action; removing `--draft --prerelease` is what would make a tag publish
   directly.

`packaging/` holds draft Homebrew and Scoop manifests. They are not
published, and their checksums must be regenerated from the real release assets
before anyone does publish them.

Publishing anywhere — crates.io, a GitHub release, a package-manager
repository — is a maintainer action. The automation builds and verifies; it does
not publish on its own beyond the tag-triggered prerelease.

## Benchmarking

`docs/benchmark.md` records the method and the raw data. In short: use
`hyperfine --shell none` with warmups, compare instruction counts because wall
clock on a shared machine is not reproducible, and never quote an upstream
README's speed as measurement.
