# v0.1 validation record

Dates: 2026-10-10 to 2026-10-11 (Asia/Shanghai). Checkout: `/Users/leo/leocode/mine/justfetch`, branch main, starting commit f255182 (clean working tree containing LICENSE and empty README). All implementation changes remain local and uncommitted. No push, release, package publication or remote CI invocation occurred.

## Native macOS checks

Homebrew rustc 1.99.0 (b940084d7), cargo 1.99.0 (5f94df478), aarch64-apple-darwin. Actual commands:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

All passed after fixes. Test count: **27** as of this round (10 library/parser/policy tests, 6 actual CLI tests, 2 Unix pseudo-terminal tests, 9 fixed-snapshot rendering/model tests), zero failures and zero ignored tests. Later rounds below raise this; the current total is stated in the most recent section. Production paths have no unwrap/expect, external commands or runtime network access. Initial empty-CLI tests failed in all four contract groups; implementation made them pass. Sanitization tests first exposed raw ESC passthrough, then passed with the state-machine sanitizer.

The PTY tests exercise actual TTY color behavior, non-UTF-8 NO_COLOR, explicit no-color, TERM=dumb and detected widths of 1, 25 and 80 columns. Fixtures separately cover tall/short logos, long CPUs, missing/error/unsupported/estimated states, display-cell widths, combining marks, emoji sequences, native numbers in JSON and hostile control sequences. CLI tests parse the entire stdout as one JSON document and check stderr/exit behavior.

Actual ordinary-user Release execution on Apple M3 / Mac15,12 / macOS 26.6.2 returned available OS, Host, Kernel, Uptime, CPU, Memory and current-parent zsh. The development tool sandbox independently denied kern.boottime; the error remained confined to Uptime while other fields and JSON still worked. Sandboxed/runner launches can also have an unrecognized parent and correctly use labeled default-shell fallback. The Release binary requires no administrator privileges.

## Cross-target checks

Installed Rustup standard-library targets and used the Rustup stable compiler **1.98.1**, explicitly selected because PATH initially resolved Homebrew rustc 1.99.0. A failed first attempt mixed the Homebrew compiler with Rustup target libraries and could not find core/std; selecting the matching compiler resolved that environment issue.

```sh
RUSTC=/Users/leo/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  cargo check --locked --all-targets --target aarch64-unknown-linux-gnu
# same command with:
# x86_64-unknown-linux-gnu
# x86_64-pc-windows-gnu
# x86_64-apple-darwin
```

All four target checks passed. These are compilation/metadata checks, not target linking, execution or hardware validation. Native Linux, Windows MSVC, WSL1/2 and Intel Mac detector behavior remain untested locally. Cross-platform Clippy additionally passed with warnings denied for aarch64-unknown-linux-gnu, x86_64-pc-windows-gnu and x86_64-apple-darwin. Compiler/Clippy were selected together via PATH to the Rustup stable toolchain.

## Independent review and corrections

A separate reviewer inspected actual uncommitted source, pinned sysinfo 0.39.6 implementations, tests and documentation. Fixed findings:

1. sysinfo's selected-PID refresh still enumerated all PIDs twice on macOS/Windows. Shell detection now uses direct Unix parent APIs; Windows uses one documented ToolHelp parent-ID snapshot, stops at our process, and queries only the parent image with limited rights. Owned native handles close on every return path.
2. Windows CPU list length covered only one processor group. Logical count now uses GetActiveProcessorCount(ALL_PROCESSOR_GROUPS); API failure is a field error.
3. A non-Unicode NO_COLOR value was lost by env::var. var_os and OsStr emptiness now handle it; unit and actual PTY regressions pass.
4. Cross ARM64 Clippy found c_char signedness differences; byte conversion now handles both u8 and i8 without platform-specific casts.

Reviewer follow-up found no further substantive defects in these corrections. Windows >64-CPU behavior is source/API-reviewed and compiled, not hardware-tested. README links and metadata are checked before final handoff.

## CI and package preparation

.github/workflows/ci.yml defines Ubuntu/macOS/Windows fmt, Clippy, tests, Release build and native JSON smoke execution, plus Rust 1.95 MSRV check. It has not been executed remotely because this task does not authorize pushing or publishing. `cargo package --allow-dirty --locked` passed local package verification (source archive and extracted-package build). Initial package verification was blocked by the sandbox's Cargo-index proxy; that is an environment failure, not a passing package check.

## Performance and licensing

[performance.md](performance.md) documents 600 measured ordinary-user runs (10 warmups and 100 samples for each of six commands), all recorded exits zero, with raw Hyperfine JSON. [third-party.md](third-party.md) lists exact dependency license metadata, including target-specific crates and Unicode notices. Existing LICENSE is unchanged. All logos were authored locally; upstream repository research is design reference only.

## Final verification additions

- Cross-target Clippy: passed on Linux ARM64, Windows x86-64 GNU and Intel macOS, all targets, warnings denied.
- Cargo source-package verification passed; no upload or publication occurred. The final package includes raw benchmark evidence so its local documentation links resolve.
- README/docs local Markdown links checked in both the checkout and extracted source package: no missing targets. The verified package contains 51 files, including the CI workflow and benchmark evidence.
- `git diff --check`: clean. `git diff --exit-code -- LICENSE`: unchanged.
- Final Release binary size: 798,288 bytes. Runnable at `target/release/justfetch`.
- Four-platform native hardware coverage and remote CI remain pending; the MSRV job is configured but not locally executed.

## Second review round (2026-10-11)

An independent pass re-ran every claim above against the actual checkout rather than trusting the first record. Nothing below changes an implementation decision except the one fix noted.

### Claims re-verified as true

- Cross-target checks reproduce. `RUSTC=~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc cargo check --locked --all-targets --target <t>` passes for `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-gnu` and `x86_64-apple-darwin`. A first attempt failed with `can't find crate for std`; the cause is the documented one — PATH resolves Homebrew rustc 1.99.0, which does not know the Rustup targets — and it is an environment error, not a code error.
- The macOS memory figure is not fabricated and matches its documented formula exactly. A C program calling `host_statistics64` directly computed `(internal_page_count - purgeable_count + wire_count + compressor_page_count) * page_size = 13953368064`, and `justfetch --format json` reported `used_bytes = 13953368064` in the same window.
- Tool versions in `docs/performance.md` are accurate: `fastfetch 2.69.0`, `hyperfine 2.0.0`, and `/opt/homebrew/bin/pfetch` reports `pfetch-rs 3.0.2`, so the "pfetch-rs" label is correct rather than the shell `pfetch`.
- Dependency MSRVs in `docs/research.md` match the registry manifests: sysinfo 0.39.6 declares 1.95, clap 4.6.7 declares 1.85, serde 1.0.229 declares 1.56, serde_json 1.0.151 declares 1.71, unicode-width 0.2.2 declares 1.66, unicode-segmentation 1.13.3 declares 1.85, anstyle 1.0.14 declares 1.66, libc 0.2.190 declares 1.65, windows-sys 0.61.2 declares 1.71.

### Defect found and fixed

The renderer emitted an empty styled span — an opening escape pair immediately followed by a reset — for every row below a logo shorter than the information column. Measured on the built-in macOS logo this is 6 of 78 escape sequences per invocation, and the count grows for shorter logos. Visible output was unaffected, so the existing sanitize-equality tests could not see it. `theme::accent` now returns empty text unchanged.

The regression test was written twice. The first version used a logo *taller* than the information column and passed with the fix reverted, which proves it tested nothing; the second version also asserted the wrong invariant, because anstyle renders one span as several escape sequences, so adjacency alone is normal. The committed version detects a reset sitting directly where a preceding escape ended, and was confirmed to fail with the fix reverted and to pass with it restored. A reverted-source run only counts as a control if the rebuild actually happened: `sed -i.bak` plus `mv` restores a file with an older mtime, Cargo then treats the library as fresh, and the first reverse test silently measured the unmodified binary.

### Documentation corrected

- `docs/compatibility.md` claimed long fields "wrap without dropping data". They wrap on grapheme boundaries, but a grapheme wider than the columns left for it is replaced by `?`; `wrap("中", 1) == ["?"]` and `wrap("👩🔬", 1) == ["?"]`. The hard column bound is deliberate, so the text was corrected rather than the code.
- `docs/compatibility.md` described the logo threshold as "the two-column threshold", which names no actual rule, and implied a hostname fallback on macOS that does not exist. Both are now stated as implemented and confirmed against the source.
- `docs/performance.md` gained the re-measurement below.

### Re-measurement that did not reproduce

The documented wall times could not be reproduced. On an identical Release binary (798,288 bytes, same size before and after the fix) peak RSS agreed to within 0.05 MiB for all six commands while wall time came out roughly 3x lower across the board, including `--help`, which performs no collection. A follow-up attempt during a busy period (load average 11.6 / 29.7 / 31.7) gave a `/usr/bin/true` baseline of 31.5 ± 26.9 ms and was discarded. The ranking also reversed: pfetch-rs was faster than justfetch on 2026-10-10 and slower on 2026-10-11. The earlier numbers are now recorded as an upper bound measured under load. Peak RSS is the reproducible part.

### Still not verified

- **MSRV 1.95.0 is still not executed locally.** `rustup toolchain install 1.95.0 --profile minimal` was attempted three times. The first two aborted mid-download (`unexpected EOF`, then `dns error: failed to lookup address information`), leaving a partial toolchain directory containing `lib/rustlib` but no `bin/`. The third resumed from the retained `.partial` files and did advance, but measured 479 KB in 45 s — about 10.6 KB/s — against roughly 80 MB still outstanding, so it was stopped rather than left running for well over an hour. Note that the first failure also reports the reason a bare `cargo +1.95.0` cannot be used here: PATH resolves Homebrew cargo, which is not a Rustup shim.

  The strongest evidence available without the toolchain is indirect but concrete: sysinfo 0.39.6, the highest-MSRV dependency, declares `rust-version = "1.95"` in its registry manifest, and every other direct dependency declares less. The declared floor therefore cannot be lowered and is exactly the binding constraint. That is a manifest review, not a compile check, and the CI `msrv` job remains the first real test of it.

  Roughly 85 MB of Rustup `.partial` files remain in `~/.rustup/downloads`. They are Rustup's own cache and a later `rustup toolchain install 1.95.0` resumes from them; they are safe to delete if that space is wanted.
- **Linux and Windows still have no native run.** A Lima Debian VM was attempted with user approval. The image download failed with `unexpected EOF` under the tool sandbox and could not be retried on this connection, whose throughput measured around 1 MB/min. No Linux or WSL detector has been executed.
- **The benchmark environment is not quiesced.** See above; a publishable wall-time baseline needs a quiet machine.
- `docs/benchmarks/macos-arm64.json` is unchanged, since the only re-measurement available was taken under contention and replacing a consistent record with a noisier one would be a downgrade.

## Code review round (2026-10-15)

A two-axis review ran against the whole working tree with two independent reviewers, one for the repo's documented standards and one for the originating spec. Every finding was re-checked against the source before being accepted; one Spec finding was rejected as wrong.

### Fixes applied

1. **Panics in production paths.** `src/platform/linux.rs` reached `unreachable!` from `collect()` in two places via `Detection::map` on variants that cannot carry a value. Both now match the non-`Available` variants explicitly. This also makes the first-round claim "production paths have no unwrap/expect" true of panic macros as well.
2. **Unchecked FFI result.** `src/platform/macos.rs` discarded the `kern_return_t` from `mach_port_deallocate`, so a send right that could not be released was reported as a clean read. The result is now checked and reported.
3. **Duplicated uname identity.** The hostname and kernel construction was byte-identical in the Linux and macOS adapters. It moved to `platform::uname_identity`, which is compiled on every Unix target and now has direct unit tests for both the success and failure paths — coverage the platform modules previously could not have. The Linux adapter still cannot be executed here, but the extracted code is exercised on macOS.
4. **Stringly-typed logo control flow.** `terminal::render` decided the Linux fallback by comparing `logo.name == "fallback"`. `logo::select` now takes the platform family and makes that choice itself, and the branch that had no test coverage now has one.
5. **Duplicated field-key list.** The seven keys were listed twice, once to build rows and once to decide which rows get a styled prefix. Both now read one `FIELD_KEYS` constant, and `KEY_PREFIX_LEN` is tied to the prefix format by a test instead of by a bare `8`.
6. **Copied logo artwork.** `assets/logos/linux.txt` was the widely-circulated Tux ASCII art of unknown authorship while the adjacent notice claimed every file was authored for justfetch. The file was replaced with an original drawing and the provenance statement corrected; `docs/third-party.md` gained an asset section.

### Rejected

- A Spec finding called the `identity` object (username@hostname) unrequested scope creep. It is not: the spec's default-layout diagram requires a `username@hostname` title line, and the spec separately requires Host to distinguish device model from host identity. The finding came from an incomplete spec excerpt handed to that reviewer, not from the code.
- A standards finding called `mem.total_bytes.max(1)` a silent clamp. All three detectors reject a zero or absent total before constructing `MemoryInfo`, so the guard is unreachable; the documented rule it was matched against concerns clamping counter values into plausible ones, not guarding a division.

### Current totals

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` and `cargo build --release --locked` all pass. Test count is now **35**: 13 library, 6 CLI, 2 pseudo-terminal, 14 rendering/model. Cross-target Clippy with warnings denied passes for `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-gnu` and `x86_64-apple-darwin`, which is the only check available for the platform code changed here.

### Known gap this round did not close

The benchmark still has no Macchina measurement and no reproducible wall-time baseline, so the spec's "benchmark against Fastfetch, Macchina and pfetch-rs" remains only partly satisfied. Both the missing comparator and the wall-time instability are recorded in performance.md rather than papered over.
