# v0.1 validation record

Dates: 2026-10-10 to 2026-10-15 (Asia/Shanghai). Checkout: the repository root, branch `main`, starting commit `f255182` (a clean working tree containing `LICENSE` and an empty README). The sections below are chronological and each states its own scope: the early rounds ran entirely locally, with no push, release or remote CI invocation, and the later sections record the pushes and CI runs that followed.

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
RUSTC=~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
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

[benchmark.md](benchmark.md) documents 600 measured ordinary-user runs (10 warmups and 100 samples for each of six commands), all recorded exits zero, with raw Hyperfine JSON. [third-party.md](third-party.md) lists exact dependency license metadata, including target-specific crates and Unicode notices. Existing LICENSE is unchanged. All logos were authored locally; upstream repository research is design reference only.

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
- Tool versions in `docs/benchmark.md` are accurate: `fastfetch 2.69.0`, `hyperfine 2.0.0`, and `/opt/homebrew/bin/pfetch` reports `pfetch-rs 3.0.2`, so the "pfetch-rs" label is correct rather than the shell `pfetch`.
- Dependency MSRVs in `docs/research.md` match the registry manifests: sysinfo 0.39.6 declares 1.95, clap 4.6.7 declares 1.85, serde 1.0.229 declares 1.56, serde_json 1.0.151 declares 1.71, unicode-width 0.2.2 declares 1.66, unicode-segmentation 1.13.3 declares 1.85, anstyle 1.0.14 declares 1.66, libc 0.2.190 declares 1.65, windows-sys 0.61.2 declares 1.71, toml 1.1.8 and its chain declare 1.85.

### Defect found and fixed

The renderer emitted an empty styled span — an opening escape pair immediately followed by a reset — for every row below a logo shorter than the information column. Measured on the built-in macOS logo this is 6 of 78 escape sequences per invocation, and the count grows for shorter logos. Visible output was unaffected, so the existing sanitize-equality tests could not see it. `theme::accent` now returns empty text unchanged.

The regression test was written twice. The first version used a logo *taller* than the information column and passed with the fix reverted, which proves it tested nothing; the second version also asserted the wrong invariant, because anstyle renders one span as several escape sequences, so adjacency alone is normal. The committed version detects a reset sitting directly where a preceding escape ended, and was confirmed to fail with the fix reverted and to pass with it restored. A reverted-source run only counts as a control if the rebuild actually happened: `sed -i.bak` plus `mv` restores a file with an older mtime, Cargo then treats the library as fresh, and the first reverse test silently measured the unmodified binary.

### Documentation corrected

- `docs/compatibility.md` claimed long fields "wrap without dropping data". They wrap on grapheme boundaries, but a grapheme wider than the columns left for it is replaced by `?`; `wrap("中", 1) == ["?"]` and `wrap("👩🔬", 1) == ["?"]`. The hard column bound is deliberate, so the text was corrected rather than the code.
- `docs/compatibility.md` described the logo threshold as "the two-column threshold", which names no actual rule, and implied a hostname fallback on macOS that does not exist. Both are now stated as implemented and confirmed against the source.
- `docs/benchmark.md` gained the re-measurement below.

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

The benchmark at that point had no Macchina measurement and no reproducible wall-time baseline, so the spec's "benchmark against Fastfetch, Macchina and pfetch-rs" was only partly satisfied. Both were recorded in benchmark.md rather than papered over. **Closed on 2026-10-16; see the benchmark section below.**

## First CI run (2026-10-15)

Dated locally, as the rest of this record is: GitHub timestamps the run `2026-10-14T17:12Z`, which is 2026-10-15 01:12 in Asia/Shanghai, so this section still follows the one above it in time.

Pushing `main` to `origin` triggered the workflow for the first time. Every job passed: `macos-latest / stable`, `ubuntu-latest / stable` (40s), `windows-latest / stable` (2m21s) and `MSRV / 1.95.0` (17s), run [`38604699769`](https://github.com/LightYourJourney/justfetch/actions/runs/38604699769). Each native job ran format, lint with warnings denied, tests, a release build and a real `--format json` invocation.

This closes two items the previous rounds had to record as unverified.

**MSRV 1.95.0 is now verified.** `cargo +1.95.0 check --locked --all-targets` passes on the CI toolchain, after three failed attempts to install that toolchain locally on this machine. The manifest review recorded earlier (sysinfo 0.39.6 declaring `rust-version = "1.95"`) turned out to agree with the compile check, but it was the compile check that settled it.

**Linux and Windows now have real detector runs**, not just cross-compilation. The release binary executed on both runners and returned all seven fields as `available`:

| Field | Ubuntu 24.04, kernel 6.17.0-1022-azure | Windows Server 2025 Datacenter, build 26100 |
| --- | --- | --- |
| OS | `Ubuntu 24.04` from os-release | `Windows Server 2025 Datacenter` from the registry |
| Host | `Virtual Machine`, kind `device_model` (DMI) | `runnervmfi6oq`, kind `hostname` |
| Kernel | `Linux 6.17.0-1022-azure` | `Windows NT build 26100` |
| Uptime | 186 s | 406 s |
| CPU | `INTEL(R) XEON(R) PLATINUM 8573C`, 4 logical | `AMD EPYC 9V74 80-Core Processor`, 4 logical |
| Memory | 16,765,374,464 B total, `total_minus_available` | 17,174,360,064 B total, same definition |
| Shell | `bash`, `current_parent` | `cmd`, `configured_default` |

Two things this settles. The Linux parent-shell path (`getppid` plus `/proc/<pid>/exe`) resolves correctly on a real kernel, and the Windows native paths (registry product name, `GlobalMemoryStatusEx`, ToolHelp parent snapshot) work on a real Windows system. The Windows shell line is not a defect: `cargo run` replaces itself with `exec` on Unix, so the parent there is `bash`, but on Windows it must spawn a child, so the parent is `cargo.exe`, which is not a shell — and the field correctly fell back to the labelled `ComSpec` default instead of guessing. That is the documented immediate-parent-only semantics working as designed.

**What the run does not establish.** A GitHub runner is one configuration per platform, not a distribution matrix. Arch, Debian, Fedora, Gentoo, openSUSE and NixOS detection, Intel Macs, ARM64 Linux, WSL1/WSL2, containers and any hardware-sensitive path remain unexercised by that run (later rounds record the narrow ARM64 exception), and no runner exercises a real interactive terminal or the `--logo none`/color combinations against a TTY. The compile-only rows in the compatibility matrix that are still compile-only are marked as such. The benchmark gap is unchanged by a CI run, since CI measures nothing about startup time.

## Benchmark round (2026-10-16)

Macchina 6.4.0 was installed and measured, closing the comparator the spec asked for and the two rounds above had to leave open. The run also settled why the wall times never reproduced.

**Method.** Hyperfine executes one command's samples consecutively, so a load spike biases that entire command. Ten rounds of ten runs per command were run with the command order rotated every round, pooling 700 samples across a run whose load average went from 7.4 to 36–41. The machine was deliberately not quiesced. Raw output is in [benchmarks/macos-arm64-interleaved-2026-10-16.json](benchmarks/macos-arm64-interleaved-2026-10-16.json).

**Result.** Wall-clock coefficient of variation across the seven commands was 59%–125%; CPU time 17%–21%; instruction counts **1.2%–2.1%**. Wall clock on this host measures the machine, not the program. The baseline was therefore moved onto instruction counts, with wall clock retained only as a load-contaminated upper bound, and the round's medians are 27.8 M instructions for a full justfetch run against 34.2 M pfetch-rs, 41.8 M Fastfetch and 46.0 M Macchina.

**A hypothesis was tested and failed.** The note written on 2026-10-11 guessed that a uniform per-run harness cost explained the 3x wall-time drift. Measuring the same four commands sandboxed and unsandboxed minutes apart showed the sandbox is not the cause — the unsandboxed run was slower and far noisier because the machine had become busier. Host load explains it; the earlier guess was wrong and `docs/benchmark.md` now says so.

**A finding worth acting on later.** `justfetch --help` costs 27.4 M instructions and a full run 27.8 M. Collection and rendering are about 1.5% of the process; the rest is process creation, dynamic linking and argument parsing. No detector change can move the remaining 98.5%, so future performance work that matters would have to target startup, not collection.

**Corrections made during this round.** Two claims of mine were wrong and were fixed before commit. The first, in `docs/benchmark.md`, was the harness-cost inference above. The second was a sentence asserting Macchina "has no such key in its binary" for hiding ASCII art: a wider search of the binary found `hide_ascii` and `prefer_small_ascii` theme keys, so the sentence was replaced with what is actually true — no CLI flag exists, the theme schema is undocumented, `-t` silently falls back on an unknown name, and `XDG_CONFIG_HOME` is ignored on macOS, so using a theme would mean writing into the reader's own config directory. Macchina was measured as it ships and its figures are labelled an upper bound rather than a like-for-like ratio.

**What this round does not establish.** The host was never quiesced, at the operator's explicit request, so no wall-clock figure here should be published as a baseline. Instruction counts do not capture I/O waits or terminal painting. Nothing here is evidence about Linux or Windows performance, and peak RSS is Hyperfine's process peak-resident metric, not retained heap.

## Release round (2026-10-16)

v0.1.0 was published as a GitHub pre-release from a new release workflow, with archives for five targets. The workflow was exercised through its `workflow_dispatch` trigger before any tag existed so that a failed build would cost no tag; run [38855740216](https://github.com/LightYourJourney/justfetch/actions/runs/38855740216) built all five, skipped the release job as designed, and its artifacts were downloaded and inspected: each archive holds the binary, `LICENSE` and `README.md`, and `file` reports the expected architecture for all five (Mach-O arm64, Mach-O x86_64, ELF x86-64, ELF aarch64, PE32+ x86-64). The two macOS binaries were executed. `actionlint` reports no findings on the new workflow.

**A defect the artifact run found.** The cross-compiled Intel macOS binary reported 3.4 GiB of used memory where the native binary reported 13.7 GiB and `vm_stat` agreed with the native figure. The ratio was exactly 4.0, and it is the page-size ratio: Rosetta reports `sysconf(_SC_PAGESIZE)` as 4096 to an x86_64 process while the Mach counters from `host_statistics64` are in the host's 16384-byte pages. Measuring the three candidate sources settled which to trust rather than guessing — `sysconf` returns 4096 under Rosetta, `vm_page_size` also returns 4096, and only `host_page_size()` returns the true 16384. The detector now calls `host_page_size()`, and the fix was verified by running the rebuilt x86_64 binary against the arm64 one four times in alternation: they agree to within 6 MiB, where they previously differed by a factor of four. The pre-existing plausibility check (`used <= total`) is satisfied by a four-fold undercount, so it could not have caught this.

**Not verified here.** No Intel Mac, no ARM64 Linux machine, no WSL and no non-Ubuntu distribution was available, so those columns in the compatibility matrix are unchanged and the release is a pre-release for that reason. The five archives are unsigned. The release workflow has not yet run from a tag; only its dispatch path has been exercised.

## Field round (2026-10-23)

Four fields were added — Swap, Disk, Terminal and Local IP — chosen by comparing the module lists of the installed Fastfetch 2.69.0 (76 modules), Macchina 6.4.0 (20 readouts) and pfetch-rs 3.0.2 against justfetch's seven. They are the ones every major tool shows that can be collected on all three platforms without a subprocess, administrator access, a directory scan or a private-file read. The rest were left for a later round, and the specific reason matters more than the label:

- **GPU** needs a driver-level interface per platform, and on macOS the usual shortcut (`system_profiler`) is a subprocess this project will not run.
- **Display resolution** needs CoreGraphics on macOS, DRM on Linux and `EnumDisplaySettings` on Windows — three unrelated driver paths, not one implementation.
- **Battery** needs IOKit on macOS, `/sys/class/power_supply` on Linux and `GetSystemPowerStatus` on Windows; only the latter two are cheap, and a desktop reports nothing at all.
- **Packages** needs each distributor's package database — per-distributor files, or on macOS a walk of the Homebrew Cellar, which is the directory scan this project forbids.
- **Desktop environment and window manager** are environment strings on Linux, but macOS and Windows have no equivalent concept to report, so two thirds of the matrix would show a constant or an empty row.

**Measured on macOS 26.6.2 / Apple M3:**

- Swap matches `sysctl vm.swapusage` exactly — 13312 MiB total and 12396 MiB used from both sources.
- Disk matches Fastfetch's own Disk module exactly at one point in the round and to 0.05 GiB at another; both were the same call on the same filesystem. It does **not** match `df`'s `Used` for the same path, and that was investigated rather than smoothed over. `statvfs` returned `f_bfree == f_bavail`, so `used` is `total - free` and `used + available` happens to equal `total` on this volume. APFS reports container-wide counters, so that figure covers every volume in the container while `df` prints each volume separately: at the second measurement `statvfs` gave 317.0 GiB used and 143.4 GiB available, and the five volumes `df` listed for the same container summed to 315.7 GiB used. The 1.3 GiB difference is attributed to no volume by `df`, so it is recorded as an unexplained difference rather than labelled container overhead on a guess.
- Local IP matches `ifconfig en0`.
- Uptime dropped from 15 days to 5h50m between rounds; `sysctl kern.boottime` confirmed the machine had rebooted, so this is not a regression.

**Checks:** `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and 41 tests pass. The count rose from 35 with two swap-parser tests (units, zero total, and the rejection cases), one renderer test covering zero swap and multiple addresses, one covering an address whose adapter cannot be named — including the JSON side, where the interface is omitted rather than sent as `""` — one covering a field in the `unsupported` state, and one asserting that a lossy path label leaves the counters `confirmed`.

**Fixed after review.** A two-axis review found one defect that both axes independently reported: the Windows local-IP path filtered on `OperStatus` alone, so the loopback pseudo-adapter would have been listed as routable. Loopback, link-local and unspecified addresses are now excluded by one shared predicate used by both adapters, and the Windows path reads `sin_addr` directly instead of formatting through `inet_ntop`. An adapter the platform cannot name now omits the interface field from JSON rather than sending `""`, which rendered as empty parentheses. A non-UTF-8 home path keeps the exact counters and attaches a note that the label is lossy, rather than passing replacement characters off as the path or marking the numbers themselves estimated. Three duplicated shapes were also collapsed: the capacity format shared by memory, swap and disk; the `/proc/meminfo` line parser shared by memory and swap; and the address filter shared by the two adapters.

**Cross-target:** the Windows and Linux paths were type-checked from macOS with `cargo check --target x86_64-pc-windows-gnu` and `--target x86_64-unknown-linux-gnu`, both clean. That establishes compilation and nothing else — same caveat the project applies everywhere else. Two Windows type errors were caught this way before reaching CI: a `usize`-to-`u64` narrowing and the `PSTR` type of `inet_ntop`. Both were in code that has since been replaced, so the check's value here is the record, not the surviving lines.

**First execution on Windows (run 40104688691).** The native CI job ran the release binary after the field set grew to eleven, which is the first time these paths executed rather than merely compiled. Disk, terminal and local IP behaved as designed on that runner: disk reported 118 GiB of 149 GiB on `C:\Users\runneradmin`, terminal reported unavailable because a CI job has no terminal environment, and local IP listed `10.1.0.115 (Ethernet 4)` and `172.17.192.1 (vEthernet (nat))` with no loopback entry — the filter the review added, working on real input.

**Swap failed there, and chasing it was the useful part.** The first run reported `Inconsistent commit counters` without saying why. A second run, after the code was changed to put the counters into the message, gave the answer: commit limit 18.87 GiB, physical memory 16.00 GiB, committed 2.41 GiB, available 13.37 GiB. Committed memory is therefore *smaller* than resident memory (2.62 GiB), so the subtraction meant to yield page-file usage went negative. Once stated, that is expected — file-backed mappings and driver memory are resident without being committed — but it had to be measured to be believed. The derivation is not repairable within the APIs this project permits, so Windows swap reports `unsupported` with that reason. Run 40105894905 was the intermediate attempt; run 40107011929 is the one that confirmed the `unsupported` state on the runner.

**A guess was written down and then disproved.** The note committed between the two runs attributed the failure to a commit limit that does not exceed physical memory, i.e. a machine with no page file. The counters showed a limit above physical memory and a different cause entirely. The wrong guess stays in the record rather than being quietly deleted: what mattered was printing the values instead of guessing a second time.

**Not verified.** No Linux or Windows machine was available for interactive use, so those figures come from one CI run each, not a distribution matrix. Windows swap carries no value at all: `unsupported` with a reason is the honest state, and a number there would be a derivation this round disproved.

## Stabilisation round (2026-10-26)

A source-level audit of every detector, renderer and workflow, done against the checkout rather than against earlier rounds' conclusions. Five defects were confirmed and fixed. The rest of the audit produced no change, and the places where that is a judgement rather than a proof are listed at the end.

**Terminal.** `TERM` was reported as the terminal application with `confidence: confirmed`. It is a capability type — `xterm-256color` names what a terminal can do, not which program it is — so a host without `TERM_PROGRAM` was told it was running a program called `xterm-256color`. `TerminalInfo` now carries a `kind`: `TERM_PROGRAM` and `WT_SESSION` report `application`, and `TERM` reports `capability_type` with `estimated` and a note. The text row reads `~ xterm-256color (TERM type)`. Verified by running with `TERM_PROGRAM` unset.

**Privacy.** Nothing withheld identifying values; `--privacy` now does. Verified end to end: JSON marks the user name, host name and `local_ip` as `redacted`, the first text line is `redacted@redacted`, and the disk path becomes `~` with its counters intact. The Host row is also withheld when its kind is `hostname`, because it would otherwise repeat the name just hidden; that branch is covered by a fixture test, since this machine reports a device model.

**Rendering.** The field labels and the field values were two parallel arrays joined by index, and the styling pass decided which rows to accent by slicing eight bytes and matching label strings. Both are replaced by one ordered row list whose entries carry their own accent range, and the label column width is derived from the labels. No plugin registry, no configuration and no dynamic dispatch: the extension point is the list.

That refactor introduced a regression, which the independent review caught rather than the suite: in the branch used when the terminal is too narrow for a continuation indent, the accent was dropped, so at 8 to 10 columns the label column lost its bold colour. A width sweep from 6 to 12 columns now pins the boundary — accented at 8 and above, plain below, which is what the old string-slicing rule did. The `byte-identical` claim this round started with was too strong and is not repeated.

**CI and release.** The CI smoke step ran `cargo run -- --format json` and asserted nothing, so a collector could fail on every runner and the job would stay green. `tests/detection.rs` now asserts on each platform that the core fields are available and that the reported numbers are self-consistent, and runs as part of `cargo test`. The release workflow now executes the binary it built, for every target that can run on its own runner; the cross-compiled Intel macOS archive is excluded explicitly rather than left implied to have been tested.

**Platform audit.** Every `unsafe` block was re-read against its SAFETY comment. One gap was found and closed: the Windows local-IP path cast a `SOCKADDR` to `SOCKADDR_IN` after checking the family but not `iSockaddrLength`, so a short buffer would have been read past its end. Buffer sizes, the `size == size_of` checks after each sysctl call, the `xsw_usage` layout, the alignment of the `IP_ADAPTER_ADDRESSES_LH` buffer, and send-right ownership around `mach_host_self`/`mach_port_deallocate` were checked and needed no change.

**Performance.** The recorded baseline described a seven-field binary that no longer exists. It was re-measured on the eleven-field binary with each competitor asked for as many of the same fields as it supports: 28.2 M instructions for justfetch against 34.0 M pfetch-rs (seven fields), 42.6 M Fastfetch (eleven) and 49.7 M Macchina (ten). The four added fields cost 1.4%. The previous round's competitor numbers are kept and labelled incomparable rather than silently dropped.

**CI, run 40301536939.** All four jobs pass. The new assertions executed rather than being skipped: 49 tests on macOS and Ubuntu, 43 on Windows (the difference is the Unix-only PTY suite), with all seven new tests present in each log. That is the first evidence that `--privacy`, the terminal `kind` and `detection.rs` hold on Linux and Windows, so the earlier "not claimed until CI runs" caveat is now discharged.

**A failure the first run produced.** macOS went red in the pre-existing PTY width test: a line captured from a 1-column terminal arrived as `"4\r"`, and `sanitize` turns a carriage return into a space, so it measured two columns and failed an assertion about layout. The program does not write a meaningful CR — `sanitize` converts any in the data to a space — and a local capture has exactly one CR per LF and no CR-CR sequence, so the extra one is the PTY's own line-ending translation. The capture is now normalised before measuring. The test that would have told us this printed the entire render, which the CI log truncates; it now reports only the offending line and its measured width.

**Still not verified.** The Intel macOS archive is built and never run. Wall clock on this host moved by roughly 2x between two rounds of the same binary, so no cross-tool wall-clock claim is made in either direction.

**Independent review.** A reviewer that had not seen this round's work re-ran the checks and wrote its own experiments. It found no blocking defect and four worth fixing, all now fixed: the narrow-width accent above; `tests/detection.rs` asserting that shell detection is available, which fails on any host without `SHELL` exported (reproduced with `env -u SHELL`); an empty `WT_SESSION` being taken as evidence of Windows Terminal; and `--help` not mentioning that `--privacy` also shortens the home path. It also confirmed privacy end to end, that every exhaustive match handles `redacted`, and that the workflow shell steps are safe under `set -e`.

**Second review round.** A further two-axis review over this round's own commits found no blocking issue and five worth fixing, all now fixed. The README still said `--privacy` was planned for a later version, in a file that elsewhere documents it as shipped. `architecture.md` said every breaking schema change raises the version, while `redacted` is a new status value at version 1 — now explained rather than left silent. The PTY capture normalisation deleted every carriage return instead of only a trailing one, which would also have hidden a CR leaking from the data. The release workflow ran the build output rather than the archive it produced, and neither workflow asserted a named field, so one working detector could keep them green. `compatibility.md` still described seven fields and the older run.

The reviewer also confirmed the layering judgement — privacy as a separate pass over the snapshot, `Row`/`Line`/`Accent` as a minimal replacement for the parallel arrays rather than speculation about v0.2 — and agreed that `redacted` is the correct opposite of erasing a state.

Two regression tests came out of it: a stdout with no reader must exit successfully, and `supports_ansi` must follow the platform console instead of assuming it. The release workflow now unpacks the archive it built and runs the binary from inside it, checking that `os` and `memory` are available by name, and verifies that `SHA256SUMS` covers every archive. The assertion logic was exercised locally against a deliberately broken document to confirm it fails.

**Final independent re-evaluation.** A reviewer that had seen none of this work re-ran every check and tried to break the renderer with a PTY sweep at 1–200 columns carrying ANSI, OSC, DCS, bidi, CJK, emoji, ZWJ sequences, flags, combining marks and 500-character `USER`/`TERM_PROGRAM` values. It found **no P0**: no panic, no column overflow, no leaked escape sequence. It found several things worth fixing, all now fixed:

- The README described the eleven fields and `--privacy` as part of the published v0.1.0 pre-release, which actually contains the seven-field build. The status line now separates the published release from the unreleased work in its first sentence.
- The memory test asserted only `used <= total`, which a four-fold page-size undercount satisfies — exactly how that defect survived four green CI jobs. A test now compares against an independent reader within a factor of two, which a fourfold error fails.
- The logo test asserted names, ASCII-ness and width, so two distributions shipping the same picture would have passed. It now compares the artwork itself.
- `--help` described `--privacy` as withholding the home path, while it shortens it to `~` and keeps the field available.
- `compatibility.md` said ten fields were available on Windows (nine are; `swap` is unsupported and `terminal` unavailable in a CI job), claimed Host was the only field with a fallback chain (Shell, Terminal, Linux OS and Disk have them too), and still recorded ARM64 Linux as never run after the release workflow executed its packaged archive.
- `roadmap.md` still said v0.1 was not published.
- `Cargo.toml` omitted `release.yml` from the package while the CHANGELOG links to it, so the packaged crate carried a broken link.
- `architecture.md` said the collector owns all system access; `Config` also calls the terminal capability query.
- U+2028 and U+2029 are not control characters, but some renderers break a line on them, so they now become spaces like the ASCII line breaks. Unix disk now rejects a zero-block filesystem, matching Windows, instead of rendering `0.00 / 0.00 GiB`.

Two of its findings were stale rather than real: it reported the release workflow's execution step as never having run, and no CI result for the commit under review. Both exist — the release workflow was dispatched at that commit (run 40304542365: five builds, four packaged-binary runs, the cross-compiled Intel macOS correctly skipped) and CI passed at the same commit (run 40304508178).

**Fourth review round.** A two-axis review of the fix commit found that several of its own fixes were incomplete, and all are now closed:

- The new logo test compared logos against each other, which catches two distributions sharing a picture but not two pictures being swapped. It now names the expected asset file per id. Verified by swapping the artwork for `arch` and `debian`: the test fails with "arch is not showing its own artwork", and passes again after reverting.
- A doc comment was left attached to the wrong test by an edit that inserted a function between the comment and its own.
- The README fixed `--privacy` in `--help` and in the usage paragraph but not in the feature list, and its Features section and example carried no marker that they describe unreleased code. Both now say the published archive is older.
- The memory cross-check's comment claimed the two readers use different definitions; they use the same formula, so the bound is loose only because the reads are a moment apart, and the test catches a wrong quantity rather than a shared conceptual error.
- `compatibility.md` claimed every fallback step is labelled. Disk's is not — it reports the path it read — so the sentence now says so.
- ARM64 Linux was recorded as executed in its matrix row while two other paragraphs still listed it as unexercised, and the roadmap still treated it as a gate.
- The CHANGELOG repeated the "withholds the home path" wording, and `AGENTS.md` listed Swap without the Windows exception.
- The disk counter arithmetic moved into a pure function so the cases that must error — zero block size, zero total, more free than total — are covered by tests rather than being unreachable branches.

**Test count:** 54, up from 41. That figure is local: the most recent CI run (40304508178) covered an earlier commit's 51, and this one has not been pushed.

## Module and configuration round (2026-10-27)

Phase 10 added a module system and a TOML configuration file. `docs/configuration.md` is the reference; this records what was exercised.

**Selection-driven collection.** The claim that an unselected module is never queried was measured rather than asserted: 100 runs each with `hyperfine --shell none --output pipe`, comparing a full run against progressively smaller selections. Instruction medians: full run 28.47 M, `--modules cpu` 27.81 M (97.7%), `--modules cpu,memory` 27.91 M (98.0%), every module except `host` 28.34 M (99.6%). Raw measurements are in [benchmarks/module-selection-2026-10-27.json](benchmarks/module-selection-2026-10-27.json). The saving is real and small because process startup dominates; no wall-clock figure is quoted, for the reason recorded in benchmark.md.

**Configuration.** Paths resolve to `$XDG_CONFIG_HOME/justfetch/config.toml` or `~/.config/justfetch/config.toml` on Unix-like platforms, and `%APPDATA%\justfetch\config.toml` on Windows. Precedence was exercised at every layer with conflicting values: the command line beats `--config`, which beats the user file, which beats the defaults, and `--color`/`--no-color` override the file in both directions. Every malformed case — unknown module, duplicate module, empty list, unknown key, wrong type, malformed TOML, blank label, invalid `layout.logo`, missing `--config` file, empty `--modules` value — exits non-zero, writes nothing to stdout, and names the file and position.

**init and doctor.** `init` writes a file that parses back to the defaults, creates only the target, and refuses to overwrite. `doctor` reports the resolved path, the effective settings and any error, exits non-zero on a broken file and zero on a good one — including when no home directory exists and `--config` supplies a valid file.

**JSON compatibility.** A default run's text and JSON are identical to the previous commit: same keys, same shapes, same statuses, `schema_version: 1`. A selected subset reports `"status": "deselected"` for the rest, which is distinct from `error`, `unavailable` and `unsupported`.

**Independent review.** A reviewer that had seen none of this work compared the default output against HEAD byte for byte, probed precedence through a real PTY, and tried every malformed configuration. It found no P0. Its findings, all now fixed:

- `--privacy` rewrote a deselected module as `redacted`, contradicting the JSON contract this round introduced. Redaction now withholds only values that were actually collected, so a deselected module stays deselected and a failure stays a failure.
- The ~1.5% saving was quoted in two documents as measured in this file, which did not yet contain the measurement. It has been measured and the raw data committed; the figure is 2.3% for a one-module selection.
- `doctor` treated a missing home directory as a problem even when `--config` supplied a valid file, so a run that works was reported as broken.
- `--config` was not a global option, so `justfetch init --config PATH` — the form the error message recommends — was rejected by the parser.
- Two label keys naming the same module (`cpu` and `CPU`) were accepted with one silently winning.
- `doctor` printed a label containing only bidi controls verbatim; its check now uses the same sanitizer the renderer does, so anything that would be stripped is reported.
- With `os` deselected, the logo fell back to the generic emblem. That is the honest consequence of not querying the distribution and is now documented rather than changed.
- Smaller corrections: the test count, the claim that only one status value had been added, `research.md` still saying TOML was omitted, and the dependency-MSRV list missing the new crate.

**Not verified.** No Windows or Linux machine was available interactively. The configuration paths, `%APPDATA%` resolution and `--config` were exercised on macOS only, and the other platforms are covered by cross-target compilation and by CI rather than by a person using the binary there.

**Two-axis review of this round.** A Standards review and a Spec review ran independently against the same diff. Both axes reported; everything they raised is closed:

- Several documents still described the pre-round state: `architecture.md` listed TOML as roadmap while `README.md` claimed it, `CONTRIBUTING.md` and the roadmap repeated the same error, the README's status list omitted `deselected`, and `configuration.md` promised a "file and position" for errors that only carry a key. All corrected against what the code actually does.
- `--config` replaced the user file instead of layering over it, while the spec asks for inheritance through the chain. It now layers: keys the explicit file sets win, keys it omits fall back to the user file. Covered by `an_explicit_file_is_layered_over_the_user_one`.
- The file format had no way to colour the title and labels, so `[color] accent` was added with the same basic-colour vocabulary as the rest, plus `parse_colour` and a renderer test that the accent reaches the text.
- `config.rs` held both the configuration format and the two subcommands' output. They change for different reasons, so `init`, `doctor`, `Finding` and the template moved to `src/commands.rs`; `config.rs` keeps the schema, paths, parsing and precedence.
- Running `doctor` by hand showed a defect no reviewer had raised: a missing `--config` file produced two problem lines, one from the existence check and one from resolution, saying the same thing. A normal run stops at the first, so `doctor` now does too, and `doctor_reports_a_missing_config_file_once` fails with two matches when the fix is reverted.
- Smaller: the help text now has a test asserting it lists every module id, `LogoMode` gained `Display`, and the config-path resolution moved into a pure `config_base` so a Unicode path and an `XDG_CONFIG_HOME` override are covered without touching the real user directory.

**Cross-target finding.** Collapsing the duplicated `if wanted(...) { ... } else { deselected }` blocks into `if_selected` replaced the `wanted` import in the Linux and Windows adapters, which still call it. The macOS build could not see this: those files are `#[cfg]`-excluded on the host. Only the cross-target check caught it — `error[E0425]: cannot find function 'wanted'`. The same blindness hid six redundant-closure warnings that clippy reported only for the host target. Both classes are now fixed in all platform files, and the cross-target clippy runs are part of the check list, not an afterthought.

**CI.** Run [40450257128](https://github.com/LightYourJourney/justfetch/actions/runs/40450257128) on commit `ac95ac2` passed all four jobs — MSRV 1.95.0, Ubuntu, Windows and macOS — including the format, lint, test, release-build and live-JSON steps. This is the round's first real execution of the Linux and Windows adapters: the `wanted` import breakage described above failed only the cross-target check locally, and those two jobs are where it would have surfaced had it reached the push.

**Test count:** 87, up from 54 at the start of the round and 79 before the review.

## Visual, theme and layout round (2026-10-27)

Phase 11 added a theme model, four layouts and custom artwork. `docs/themes.md` is the design specification; this records what was exercised and by what machine.

**Colour capability.** Each depth was exercised on a real pseudo-terminal, not asserted from the code: `COLORTERM=truecolor` produced `ESC[38;2;136;192;208m`, a `xterm-256color` terminal produced `ESC[38;5;110m`, a plain `xterm` produced `ESC[37m`/`ESC[97m`, and `NO_COLOR=1` produced no escape at all. The pty test suite pins the 256 and 16 cases; true colour was checked by hand because the test helper clears `COLORTERM` for determinism. The 16-colour reduction is an approximation against the xterm palette, which terminals may redefine, and is documented as such rather than presented as exact.

**Layouts.** All four were rendered at every column count from 0 to 260 plus 1 000, 4 096 and 100 000, in colour and plain, with and without Unicode, against a fixture containing a 200-character CPU name, a 200-character hostname and a 200-character error. Every visible line stayed within the requested width. Rules and swatches disappear below 24 columns; artwork disappears in `classic` below `logo width + 39` and in `vertical` when it is wider than the terminal; no layout was observed to drop a row.

**Text is identical under styling.** For every built-in theme, and for every layout at eight widths, the styled output with escapes removed is byte-identical to the plain output, apart from the swatch row, which is the theme's own palette and whose length differs between themes. This is the invariant the renderer is built around: widths are measured on plain text and styles are attached afterwards.

**Logo files.** Oversized, over-long, over-wide, empty and escape-bearing inputs are covered by unit tests. Escape sequences are stripped rather than rejected; the size, line and column limits are errors. Trailing spaces are removed and leading spaces kept, so artwork alignment survives.

**JSON.** `--theme nord --layout vertical` with `--format json` produces the same document shape as a default run, with no escapes; `schema_version` stays 1.

**Default output.** At 24 columns and above, a default run's plain text is byte-identical to the previous commit's, because the default theme leaves every role unset and the accent falls back to the artwork colour. Compared field by field on stable modules (`os`, `kernel`, `cpu`, `shell`) against a build of `e4317a7` in a separate worktree: identical for classic, compact, minimal, vertical, `--logo arch` and `--logo none`. Two differences are deliberate: below 24 columns the rules and swatches are now dropped rather than squeezing out the information, and in colour mode the separator is no longer part of the styled span, so escape placement differs while the visible text does not.

**Independent review.** Two reviewers that had taken no part in the design examined the working tree — one against the specification, one for terminal compatibility and maintainability — running the binary rather than reading it alone. They found ten issues; all are closed:

- Diagnostics could carry escape sequences. A configuration value or a `HOME` containing one reached stderr and `doctor` verbatim — a config error printed an OSC-52 clipboard sequence. The first fix sanitised whole messages line by line, which stopped the escapes but kept newlines, and a second reviewer showed that this let a value forge a diagnostic: `modules = ["evil\u000Aproblem: forged"]` produced a line reading `problem: forged`. Values are now sanitised onto one line and only the TOML parser's own message keeps its line structure, because its echoed source and caret are part of the diagnostic. Reproduced with a hostile module id, a hostile `HOME` and a newline-bearing value before each fix.
- Vertical artwork was centred line by line, which compressed an asymmetric drawing: the arch's staircase began at columns 43, 42, 40, 39, 37, 37, 35 instead of 41, 40, …, 35. The block is now offset once by the artwork's width.
- `justfetch themes` guessed the terminal itself instead of using the resolved settings, so `--no-color` and `[color] enabled=false` were ignored, a non-UTF-8 locale still received `●`, and a 40-column terminal received 92-column lines. It now takes the same colour depth, Unicode decision and width a normal run uses. Descriptions wrap at word boundaries rather than mid-word, and below the width the name column and a description can share, each entry becomes a small block. Twenty columns is covered by a test.
- Custom artwork bypassed the non-UTF-8 downgrade, so a CJK logo arrived as mojibake beside `?`-replaced text. Artwork now goes through the same conversion as every other string.
- Logo precedence was wrong: `--logo none` and `--logo arch` could not displace artwork inherited from a configuration file, and `--logo arch --logo-file x` silently used the file. A tighter layer now replaces inherited artwork, and asking for two logos on one command line is an error.
- A custom theme's `name` reached `doctor` unsanitised. It is now refused if it contains control characters, like a label would be.
- The claim that a themed run is textually identical to a plain one was true of every line except the swatch row, which is the theme's own palette; the documentation said otherwise. So did the claim that the default output is unchanged, which stopped being true below 24 columns when decoration began to be dropped. Both corrected.
- `--theme` could select only built-in themes, so a custom theme required a configuration file. `--theme-file PATH` closes that.
- `Colour::Indexed` was unreachable, and several aliases were accepted without being documented; the variant is gone and the aliases are documented.
- An assertion that compared themes filtered any line containing the letter `o`, which silently dropped rows such as `Host`. It now identifies a swatch row by its shape, and still passes — the information really is identical across themes.

**Second two-axis review.** The same two axes re-examined the committed work. Findings closed:

- Artwork chosen explicitly was still dropped between the width where it fits and the width where four spare columns exist: a 22-column emblem vanished from a 22-column terminal. The stacking fallback now needs only that the artwork fits, so the margin is gone and a matching test pins both sides of the boundary.
- A row narrower than its label column lost styling: the value's first fragment was coloured, later fragments were not, and the separator sat inside the value span. Continutations are now styled as the value they continue, and the separator is never part of it.
- The `classic` arrangement could not be kept once a named logo forced stacking. It is now documented that `--logo none` keeps the two-column layout and `--layout vertical` asks for stacking deliberately.
- `themes` previewed the title, label, value, rule and swatches but not the emphasis or artwork colours. The sample now shows all of them.
- The configuration was resolved twice in `main` with the same error handling; both paths call one helper. The automatic logo was chosen in two places; both call `auto_logo`. The fallback artwork was written twice; it is one constant. The width policy's magic numbers became named constants.
- Documents overstated three things: that explicitly chosen artwork is never dropped (true only while it fits), that adding a theme or a layout is free of shotgun surgery, and that a role set to `default` means the same as leaving it unset. All three now say what the code does.
- `docs/research.md` gained the visual comparison the specification asked for, since the existing table covers architecture and dependencies rather than design.

**Rejected finding.** Both reviewers noted that `--format json` still reads and validates `[theme] file` and `layout.logo_file`, so a missing presentation file fails a JSON run. That is deliberate and now documented: the configuration is validated once, the same way for every output format, and only the document's contents ignore presentation. Making validation conditional on the format would give the same file two different meanings, and a user whose configuration names a missing file has a broken configuration whether or not that run draws it.

**CI caught a test defect, not a product one.** The first run of this round failed on Windows only: a test wrote an absolute path into a TOML basic string, where `C:\Users\...` starts a Unicode escape. The test now escapes the path through a helper with its own unit test, and the configuration reference tells users to double backslashes or use a single-quoted string. The product was never wrong here; the test could not have failed on macOS, which is why it reached CI.

**Not verified.** No Windows or Linux terminal was available interactively. The colour-depth checks above ran on macOS only; the platform-specific ANSI policy is exercised by CI and by cross-target compilation, which is not the same thing. Font-dependent rendering of `●` and of bold is not asserted anywhere, and no screenshot comparison was performed.

**Two-axis review of this round.** A Standards review and a Spec review ran independently against the same working tree. Both axes reported; the findings are closed:

- `[color] accent` did not override a theme that names its title or labels; it only filled a gap. The documentation called it an override, so the code was wrong. Verified in a terminal: `--theme nord` and `--theme nord` with `accent = "red"` now emit different title bytes, where before they were identical.
- `justfetch themes` showed only the palette, so a theme could not actually be previewed. The name is now drawn in the theme's title colour and a label/value/rule sample follows the palette.
- Explicitly chosen artwork was silently dropped on a narrow terminal. A named logo now moves above the information instead of disappearing, unless the artwork itself is wider than the terminal, where it cannot be drawn; automatic artwork still gives way, as before.
- Five `expect()` calls sat on production paths, one of them in `Config::default`, which runs on every invocation. All are gone; the two colour lookups that were "impossible to fail" are computed without assuming it.
- The distribution table was written twice — once for detection, once for explicit selection — so a new distribution meant editing both. It is now one table with the two alias sets layered on top.
- Theme and logo file loading was duplicated between the configuration path and the command line; both now call one loader.
- The theme listing styled its chips with a local copy of the renderer's colour helper, and panicked on a missing theme while the JSON listing silently skipped it. Both now share one helper and skip the same way.
- Documents still described the previous state: `architecture.md` said the configuration file was the only file read and listed themes as roadmap, `AGENTS.md` and `CONTRIBUTING.md` repeated the scope error, `architecture.md` said there were two subcommands, and the configuration reference omitted `--theme`/`--layout` from the precedence list and described `color.accent` as ANSI names only.

**Rejected finding.** A Standards finding claimed the default output was not byte-identical because the swatch row was new. It is not new: `e4317a7`'s pinned snapshot contains the same `o o o o o o o o` row, and the field-by-field comparison above shows the text is unchanged. The claim in this file was corrected only where it was genuinely inaccurate — colour escape placement, not the text.

**Accepted deviation.** `--theme-file`, `--logo-file` and `--format json themes` go beyond the specification's suggested CLI list. The file flags are how a custom theme or logo is selected without a configuration file, which the spec requires; the JSON listing exists so that asking for JSON yields JSON.

**CI.** Run [40479860007](https://github.com/LightYourJourney/justfetch/actions/runs/40479860007) on `99327a7` passed all four jobs — MSRV 1.95.0, Ubuntu, Windows and macOS. The preceding run on `f8233b7` failed the Windows job alone, for the test defect recorded above: the first execution of the theme code on that platform is also the first time a Windows path met the TOML parser.

**Third review pass, run inline.** The two reviewer agents were cut off by an API quota, so this pass was done directly against the fix commit `0628cc8`. It found two things the commit had introduced, both now fixed:

- When the separator was split across two lines, the row's continuation stayed unstyled: `past_label` was only set on the branch where the separator fitted. A row at seven columns now styles its continuation as the value it continues, and the regression test fails with the old code (`the continuation lost its colour`).
- The theme preview measured its sample with a hand-counted 15/18 cells instead of measuring the text, which would drift the moment a sample changed. It is measured now.

**Fourth review pass.** Two independent axes examined the fix commit `6308765`. Both found the same two defects, which the previous pass had fixed only partway:

- A row narrower than its whole label column still lost all colour: the fragment matching began only once a line held the entire padded prefix, which never happens when the label itself wraps. Styling is now applied by cell range — the label, separator and value ranges are computed once and each wrapped fragment is split at the boundaries that fall inside it — so where the line breaks no longer matters.
- A separator split across lines had its remainder absorbed into the value span, so with `separator = " :: "` a `:` appeared in the value colour. The same range split keeps separator fragments plain.
- The preview's width check counted the space before the sample twice and approximated the chip row as `2n` cells; it now sums the parts it actually draws, so a 51-column terminal shows the sample instead of withholding it until 53.
- The sample was built from two parallel lists, one styled and one plain, which could drift; one list of parts now feeds both the drawing and the measurement.

The tests that pinned the old behaviour were rewritten rather than kept: one asserted that a narrow line must start with an escape, which only held because the separator fragment was wrongly coloured. They now assert what the code promises — the label keeps its colour at every width, and the value fragment after a plain separator is styled.

**CI.** Run [40666473049](https://github.com/LightYourJourney/justfetch/actions/runs/40666473049) on `6308765` passed all four jobs — MSRV 1.95.0, Ubuntu, Windows and macOS — so the fixes from the inline pass are executed on the platforms as well as compiled for them.

**CI caught another test defect.** The first CI run of this pass failed on Ubuntu only: a test asserted that the output does not contain `"CPU "` to prove a custom label replaced the default. The GitHub runner's processor is an `Intel(R) Xeon(R) ... CPU @ 2.80GHz`, so the *value* contains the substring the test forbade. The assertion now checks that no line starts with the label, which is what "the row's label" means; a value that merely mentions the word cannot satisfy or break it. This is the second time this round that a test, not the product, was tied to one machine's data.

### Phase 12 overall performance

Default run (16 modules, `display` excluded), 30 runs, against a build of the
pre-Phase-12 commit `03c1fef` in a separate worktree:

| Build | Median instructions | Wall |
| --- | --- | --- |
| `03c1fef` (11 modules) | 28.25 M | 9.0 ms |
| Phase 12 (16 modules) | 37.63 M | 10.4 ms |

+33% instructions, +16% wall for five new collectors. Packages accounts for
most of it (~6 M, enumerating ~900 directory entries); GPU and Battery are
under 0.4 M each, and Desktop and Locale are environment reads. No collector
made the default run slower than the module it added.

### Independent audit of the new detection

A reviewer that had taken no part in the work audited the six collectors against
the native APIs and every FFI declaration. It confirmed the macOS figures
against `system_profiler`, `pmset`, `brew list` and `defaults read`. Twelve
findings; all are closed:

- **A crash.** `CFStringGetCString` was called on whatever
  `CFPreferencesCopyValue` returned. `AppleLocale` is user-writable, and a
  number there makes that call throw an Objective-C exception, which aborts the
  process. Every Core Foundation read now checks the type id first; a unit test
  builds a `CFNumber` and asserts the guard rejects it, so the check is
  exercised without changing the user's preferences.
- **A value from an unknown.** Windows reports `BatteryFlag == 255` for
  "unknown"; that was being turned into a battery unit. It is now
  `unavailable`, distinct from "there is no battery" (128).
- **Windows GPU vendor never populated.** `DeviceID` is
  `PCI\VEN_10DE&DEV_…`, so splitting on `&` never produced a `VEN_` prefix.
  The vendor is now taken from wherever `VEN_` falls, and duplicate adapters
  are merged by `DeviceID` rather than by name.
- **A non-existent battery.** macOS publishes both bays of a two-bay portable;
  the empty one reports `Is Present = false` and was counted. It is skipped, and
  the empty-list reason now distinguishes "no internal source" from "reported
  absent".
- **Peripheral batteries.** Linux `type = Battery` also matches a mouse or
  headset; `scope` and `present` are now required, and a capacity above 100 is
  not treated as a percentage.
- **A false primary flag.** sysfs has no primary bit, so every Linux display
  claimed "not primary". The field is now `Option<bool>` and Linux reports
  `null`.
- **A null IOKit iterator.** `IOServiceGetMatchingServices` can succeed with no
  matches; the code iterated and released object 0. It now returns
  `unavailable` before touching it.
- **A wrong integer width.** `CFNumberType` is `CFIndex` (long), declared `int`.
  It works only because the value zero-extends.
- **i3 misclassified.** It is an X11 window manager, not a Wayland compositor,
  so the variant is now `WindowManager` and the documentation says so.
- **dpkg overcount.** `.list` files survive a removal that has not been purged,
  and there is one per architecture. The count now reads installed entries from
  `/var/lib/dpkg/status`.
- **A false comment** about display-count overflow, corrected to say the extra
  displays would be omitted.
- **Two stale documents**: the validation record described Windows as reporting
  per-pack batteries (it reports one aggregate), and the README example still
  said "not eleven" rows.

**CI.** Run [40681651007](https://github.com/LightYourJourney/justfetch/actions/runs/40681651007) on `e4d0b34` passed all four jobs — MSRV 1.95.0, Ubuntu, Windows and macOS — so the six collectors compile and the test suite runs on all three platforms.

**Two-axis review of Phase 12.** The Standards and Spec axes examined the six
commits independently. Both reported; the findings are closed:

- The README example claimed to be "captured from the current source" while
  showing the eleven-row output from before the round. It is regenerated from a
  real run.
- The configuration reference used `gpu` as its example of an unknown module id
  while listing `gpu` among the valid ones; it says `bogus` now.
- The Windows GPU dedup fell back to the name only in its key, while the stored
  entry kept no id, so duplicates with an empty `DeviceID` were never merged.
- macOS scaled a charge above the maximum into 100%: an invalid counter became a
  plausible figure. It reports no percentage instead. A missing `Is Charging`
  flag no longer reads as "not charging".
- The Nix package manager had no entry at all. It now reports `unsupported` with
  a reason: a store-path count is not a package count and the profiles are not
  read.
- Sway was reported as confirmed from its socket alone, which is the single
  source the module's own rule forbids. It is `estimated` with a note.
- Compatibility tables for Packages, Desktop and Locale were missing entirely
  (earlier edits had silently matched nothing); they exist now, and the display
  section states what `width`/`height` measure per platform.
- `DisplayInfo` gained a `definition` field so the JSON says whether the numbers
  are logical points, desktop pixels or a connector's preferred mode.
- Three copies of the Core Foundation string extraction became one helper, and
  the PCI vendor table that existed twice now lives in `detect::hardware` with
  tests, alongside the dpkg status counter and the NVIDIA model parser. Those
  three pure functions are how the Linux and Windows parsing paths get exercised
  on this machine at all.
- `ModuleId::DEFAULT` is now pinned by a test to be `ALL` minus `display`, so a
  new module cannot join the default list by accident.

**Test count:** 171, up from 87.

## Advanced detection round (2026-10-28)

Phase 12 adds collectors one at a time, each with its own platform evidence.

### Locale

**Implementation.** `detect::locale::process_locale` applies POSIX precedence
(`LC_ALL` → `LC_MESSAGES` → `LANG`, empty is unset, `C`/`POSIX` are real values)
and has its own tests; `build` assembles the two halves or reports unavailable
when both are absent. System sources: `/etc/locale.conf` (fallback
`/etc/default/locale`) on Linux, `CFPreferencesCopyValue("AppleLocale", …)` on
macOS, `GetUserDefaultLocaleName` on Windows.

**macOS evidence.**

```
defaults read -g AppleLocale   zh_CN
LANG                           zh_CN.UTF-8
justfetch                      zh_CN.UTF-8 (LANG), system zh_CN
```

**Not verified.** Linux and Windows paths are cross-compiled only.

### Desktop

**Implementation.** The decision is a pure function (`detect::desktop::describe`)
and the platform adapter only gathers variables. Seven unit tests cover: two
agreeing sources, one source (estimated), a compositor socket outranking the
variables, a nested session whose sources disagree, a TTY with no variables,
an unknown identifier kept verbatim, and the `DISPLAY` fallback for the session
type.

**Not verified.** No Linux desktop was available, so the adapter that reads the
environment has never run against a real GNOME, KDE or Sway session. macOS and
Windows report `unsupported` by design.

### Packages

**Implementation.** macOS counts directories in Homebrew's Cellar and Caskroom;
Linux counts entries with status `install ok installed` in
`/var/lib/dpkg/status` and directories in `/var/lib/pacman/local`; Windows counts directories in `scoop/apps`. Each
manager is its own entry with a `definition`; nothing is summed.

**macOS evidence.** Executed on a MacBook Air and compared with Homebrew's own
answer:

```
brew list --formula | wc -l   883
brew list --cask    | wc -l    44
justfetch: homebrew formulae 883, homebrew casks 44
```

**RPM and winget.** Both are reported as unsupported with a reason. The RPM
database is SQLite or Berkeley DB and this build has no driver for it; winget
has no local package database, only a manifest cache. Counting files near either
would be a number about something else.

**Performance.** 30 runs, instruction medians, raw data in
[benchmarks/packages-2026-10-28.json](benchmarks/packages-2026-10-28.json):

| Selection | Median instructions | Wall |
| --- | --- | --- |
| `--modules os` | 28.15 M | 11.5 ms |
| `--modules packages` | 34.13 M | 12.8 ms |

The cost is enumerating ~900 directory entries, each needing its type; it is
+21% instructions but +11% wall, so it stays in the default list.

### Display

**Implementation.** macOS enumerates active displays through CoreGraphics;
Linux reads DRM connectors; Windows uses GDI `EnumDisplayMonitors`.

**macOS evidence.** Executed on a MacBook Air and compared with the system:

```
system_profiler: Resolution: 2560 x 1664 (panel)
macOS UI:        Looks like 1470 x 956
justfetch:       1470x956 @60Hz (built-in)
```

The number reported is the current mode's logical size, which is what the
desktop is laid out in; the panel's native resolution is not exposed by the API
used.

**Performance, and why Display is opt-in.** The first measurement was
unacceptable: `--modules display` took 66.6 ms against 11.9 ms for
`--modules os`, about 36.4 M instructions against 28.0 M. Isolating the calls
showed the cost was `CGGetActiveDisplayList` — the round trip to the window
server — not the mode copy, since removing the latter changed nothing. A fetch
tool cannot pay that on every default run, so `Display` is not in
`ModuleId::DEFAULT`; it is selected by name, where the cost is the user's
explicit choice. On a local GUI session the call may be cheaper; the figure
here is from this host and is recorded as such.

**Not verified.** Linux and Windows are cross-compiled only. The Linux path is
marked estimated because the compositor owns the mode in use.

### GPU

**Implementation.** macOS enumerates IOKit `IOAccelerator` services; Linux reads
DRM sysfs; Windows uses GDI `EnumDisplayDevicesW`. Each adapter is its own entry,
and an adapter the platform does not name is reported by vendor and id instead of
a name invented from a table.

**macOS evidence.** Executed on a MacBook Air (Apple M3) and compared with the
system's own report:

```
system_profiler: Chipset Model: Apple M3
justfetch:       Apple M3   vendor=Apple   kind=integrated   id=AGXAcceleratorG15G
```

**Performance.** 30 runs, instruction medians, raw data in
[benchmarks/gpu-2026-10-28.json](benchmarks/gpu-2026-10-28.json): see the
table below with the other modules.

**Not verified.** Linux and Windows paths are cross-compiled only. Windows has a
known limitation: `EnumDisplayDevicesW` lists adapters attached to a display, so
a headless compute GPU is not enumerated.

### Battery

**Implementation.** macOS reads IOKit power-source descriptions; Linux reads
`/sys/class/power_supply`; Windows reads `GetSystemPowerStatus`. Every battery
the platform reports separately is its own unit with its own percentage and
state, so two packs are not averaged; Windows exposes one aggregate reading and
therefore reports one unit. A unit the platform exposes without a charge figure
reports no percentage rather than zero, and a bay reported as not present is
not a unit at all.

**macOS evidence.** Executed on a MacBook Air (Mac15,12, macOS 26.6.2) and
compared with the system's own report:

```
pmset:     InternalBattery-0  100%; charged; 0:00 remaining present: true
justfetch: InternalBattery-0  100%  full   power=ac
```

**No battery.** A machine with no battery reports `unsupported` with a reason
rather than `0%`. That path is covered by a fixture test; the desktop, container
and cloud-VM cases are the same code path but were not exercised on such a
machine here.

**Performance.** 30 runs each, instruction counts (load-resistant; wall clock on
this host is not, see benchmark.md), raw data in
[benchmarks/battery-2026-10-28.json](benchmarks/battery-2026-10-28.json):

| Selection | Median instructions | vs `--modules os` |
| --- | --- | --- |
| `--modules os` | 27.52 M | — |
| `--modules battery` | 27.87 M | +1.29% |
| `--modules os,battery` | 28.03 M | +1.88% |
| `--modules os` (GPU run) | 27.69 M | — |
| `--modules gpu` | 27.94 M | +0.90% |
| `--modules os,gpu` | 27.92 M | +0.82% |

The marginal cost of each collector is a few hundred thousand instructions —
under one percent of the process — so startup still dominates. The GPU figures
come from a second run on a busier host; the two `--modules os` baselines differ
by 0.6%, which is the run-to-run drift to compare against.

**Not verified.** Linux and Windows are implemented and cross-compiled but have
not been run: no Linux or Windows machine was available. Their sysfs and
`GetSystemPowerStatus` paths are covered by cross-target compilation only.

## Release candidate round (2026-10-28)

Phase 13 preparation for v1.0.0-rc.1. What follows was executed on this machine
or in CI; nothing is inferred from a previous phase's claims.

### Toolchain finding

The `cargo` on `PATH` is Homebrew's Rust 1.99.1, which has no std for any target
other than the host, and the rustup stable toolchain here has a broken
`rust-objcopy` (it cannot load `libLLVM.dylib`), so `strip = true` does not run
for cross-target builds made with it. The release binaries in `dist/` were
therefore built without the strip this profile asks for, and are larger than
what CI produces. CI uses its own rustup toolchain where strip works; the local
`dist/` is a preview, not the release artifact set.

### Cross-platform matrix

| Target | `cargo check --all-targets` | Release build | Executed |
| --- | --- | --- | --- |
| aarch64-apple-darwin | Yes (host) | Yes | Yes, interactively on Mac15,12 |
| x86_64-apple-darwin | Yes | Yes (cross) | Yes, under Rosetta 2 — execution, not Intel hardware |
| aarch64-unknown-linux-gnu | Yes | No — no cross linker installed | No |
| x86_64-unknown-linux-gnu | Yes | No — no cross linker installed | No |
| x86_64-pc-windows-gnu | Yes | No — no MinGW toolchain installed | No |

The Linux and Windows binaries are built natively by the release workflow on
their own runners, which also unpack and run the archive they produced. Those CI
runs are the evidence for those platforms; this machine contributes compilation
only.

### Packaging

`cargo package --locked` builds the crate from exactly the published file list:
79 files after anchoring the `include` patterns to the repository root, not counting cargo's own `.cargo_vcs_info.json` (the
unanchored `LICENSE`/`README.md` entries had been picking up copies staged for
the release archives), and it compiled from the packaged sources.

Two macOS archives were assembled locally with the release workflow's naming and
contents (binary, `LICENSE`, `README.md`), a `SHA256SUMS` was written and
verified with `shasum -a 256 -c`, and both binaries report `justfetch 1.0.0-rc.1`
when run from the extracted archive — the x86_64 one under Rosetta.

### Dependencies

The RustSec advisory database was downloaded and compared against `Cargo.lock`:
51 dependency packages, two advisories match by name (`windows`,
`hashbrown`), and both locked versions are at or above the patched version. No
known vulnerability is present as of this snapshot. `cargo audit` itself could
not fetch the database from GitHub in this environment, so the comparison was
done directly against the downloaded files and the result is reproducible from
them.

### File-size limits

The limits were checked against the file's size before reading, so an oversized
file is refused rather than loaded. A 200 KB logo and a 2 MB configuration are
both rejected with the size and the limit in the message, at about 7.7 MB peak
memory; before this change the same files were read whole (73 MB and 75 MB peak)
and only then rejected. The configuration file gained a 1 MiB limit; before, it
had none.

### Release gates

The release job now runs only for a tag push (a manual dispatch builds and
stops), checks that the tag equals the version in `Cargo.toml`, and creates a
**draft** release. Publishing the draft is a maintainer action. None of this has
been executed here: no tag exists, and creating one is not something this round
does without authorisation.

### Performance

Measured against installed fastfetch 2.70.0 and macchina 6.4.0 with their own
default outputs; commands, field counts and caveats are in
[benchmark.md](benchmark.md#phase-13-measurement-2026-10-28). This machine is not
quiesced, so the instruction counts are the figure to compare.

## Compatibility evidence round (2026-10-29)

Phase 14. The point of this round was evidence rather than features.

### Native cross-check (2026-10-29)

Six modules, run on the MacBook Air and compared with the system's own answer:

| Module | justfetch | Native source |
| --- | --- | --- |
| GPU | `Apple M3` | `system_profiler`: `Chipset Model: Apple M3` |
| Battery | `87% (discharging)` | `pmset -g batt`: `87%; discharging; 2:04 remaining` |
| Display | `1470x956 @60Hz (built-in)` | CoreGraphics mode; the panel is 2560x1664, explained in compatibility.md |
| Packages | `homebrew formulae 883, homebrew casks 44` | `brew list --formula` 883, `--cask` 44 (recorded as an observation; the comparison now declares packages not comparable, since a formula at two versions makes the two counts differ by design) |
| Desktop | `Unsupported` | macOS has no desktop environment in this sense |
| Locale | `zh_CN.UTF-8 (LANG), system zh_CN` | `$LANG`, `defaults read -g AppleLocale` |

Battery varied between runs during the day (100% on AC, 87% discharging here);
each figure matched `pmset` at the moment it was taken.

### Review-round fixes

- The schema validator could pass vacuously in two ways: a dangling `$ref`
  resolved to JSON `null`, which constrains nothing, and `additionalProperties`
  was only honoured as `false`. A dangling reference now panics, the
  `additionalProperties` schema form is validated, and a test proves a broken
  reference fails.
- Linux reported "no display" or "no adapter" when it had merely failed to read
  a connector or card. A read failure is counted, and if nothing was read the
  state is `unavailable` with the count, not `unsupported`.
- Windows display and GPU are `unavailable`, not `unsupported`: a session without
  an interactive desktop looks exactly like a machine with no monitor or
  adapter, which is the same ambiguity macOS display has.
- `scripts/verify-linux.sh` required `python3` while claiming no tool was
  required; it now says so and checks. `check-release.sh` rejects a local
  archive whose target the workflow does not build, and requires five targets in
  the workflow.

### JSON Schema v1

`docs/json-schema.json` now describes the document: seventeen module fields,
the six states, raw units in the field names, and the two optional keys
(`note` and `interface`). `tests/schema.rs` validates real runs against it —
a default run, a run with one module selected (the `deselected` variant), and a
run of the platform-dependent modules — and asserts the schema describes exactly
`ModuleId::ALL`. Five negative cases mutate a valid document and must be
rejected, so a validator that silently accepted everything would fail the test.

The validator implements only the keywords the schema uses (local `$ref`,
`oneOf`, `type`, `const`, `enum`, `required`, `properties`,
`additionalProperties: false`, `items`, `minimum`, `maximum`); adding a schema
keyword the validator does not know is a test panic rather than a silent pass.

### State semantics

The states had drifted: "no battery" was `unsupported` while "no display" and
"no graphics adapter" were `unavailable`, although all three mean the same thing
— the question does not apply on this machine. Battery and the Linux checks are
`unsupported`; macOS display, Windows display and Windows GPU are `unavailable`,
because a sandbox, a container or a session without a desktop cannot be told
apart from a machine with no hardware. The rule is written down in
`docs/compatibility.md`: unavailable means the
platform should have answered and did not; error means an API reported failure.

### Verification procedures for platforms this machine does not have

`scripts/verify-linux.sh`, `scripts/verify-windows.ps1` and
`scripts/verify-wsl.sh` run the binary beside the platform's own tools and print
both sides for comparison. They are read-only, require no tool that may be
absent, and say so when one is. They have not been run on their platforms; they
are the procedure, not evidence. Their syntax is checked here (`sh -n`, and the
PowerShell parser), and `verify-wsl.sh` exits non-zero if WSL is mislabelled or
a not-applicable module reports a value.

`scripts/check-release.sh` checks that the version, tag, release-notes path, the
tag/version gate and the draft flag agree, derives the asset name for each of
the five targets from the workflow, and — when `dist/` exists — checks that each
local archive carries the expected name and that `SHA256SUMS` verifies and
covers every archive present. It cannot check that the workflow actually
produces those names; only a real tag run can. It passes on this tree.

### CI

The release-candidate commit `5f01550` passed all five jobs on GitHub Actions:
the three native runners, the 1.95 MSRV job and the packaging job.

## Verification-tooling round (2026-10-29)

Phase 15. The evidence for this round is that the evidence tools now produce a
verdict instead of a transcript.

### Verification scripts

`scripts/compare.py` takes a justfetch document and a JSON file of the same
observations from the platform's tools, and prints one line per module with a
verdict: `match`, `mismatch`, `uncomparable` or `unsupported`. The platform
scripts build that JSON with the tools they have, write both files to a
`mktemp` directory they remove on exit, and print only non-identifying values —
counts, percentages, adapter names, resolutions and locales.

Exit status: 0 when nothing comparable disagreed, 1 on a mismatch, 2 on a setup
problem. A verdict of `uncomparable` or `unsupported` is explicitly not a pass.

Run on this MacBook Air (macOS 26.6.2, Mac15,12) on 2026-10-29:

```
module     justfetch                    platform tool                   verdict
gpu        Apple M3                     ["Apple M3"] (system_profiler)  match
battery    78% discharging              {"percent": 78, ...} (pmset)    match
display    1470x956                   —                                uncomparable
packages   homebrew formulae 883, ...  {"homebrew formulae": 883, ...} match
desktop    unsupported                 —                                unsupported
locale     zh_CN.UTF-8                 zh_CN.UTF-8 ($LANG)              match
```

Display is `uncomparable` on purpose: justfetch reports the mode's logical
points and `system_profiler` reports the panel, so a comparison would
manufacture a mismatch.

### Independent schema validation

`check-jsonschema` 0.39.0 — the standard Python validator, not the hand-written
one in `tests/schema.rs` — accepts a full document and a partial one, and
rejects a document with an unknown state. CI now runs that validator on the
built binary's output, plus the rejection case, so the two implementations have
to agree.

### Installation

`cargo install --path . --locked` was run from this tree: it installs
`justfetch 1.0.0-rc.1`, and the installed binary reports that version and emits a
document with `os` and `gpu` available.

### Review-round fixes

- `compare.py` now reads UTF-8 with or without a BOM: Windows PowerShell 5.1's
  `Out-File -Encoding utf8` writes one, which made the documented Windows
  command fail at the comparison stage. The script writes its own JSON through
  `UTF8Encoding($false)` as well.
- The Windows fallback that runs when Python is missing used to dump the whole
  document, including `identity` — user and host name — while the script claimed
  to print only non-identifying values. It prints the six module statuses and
  the platform observations now.
- `compare.py` distinguishes a mismatch (1) from bad input (2): malformed
  documents, wrong native shapes and missing files are rejected instead of
  crashing into exit 1. A `native` GPU given as a string used to iterate its
  characters and report a false match.
- `verify-macos.sh` skips packages when `brew` is absent instead of throwing.
- The CI rejection case changes only a deselected module's `status`, so it fails
  on the state enum and nothing else.
- The roadmap had the v0.4 and v1.0 sections twice; the stale copy is gone.
- A committed `__pycache__` file was removed and ignored.

### Second review round

- The comparison was one-directional: a document with an invented extra adapter
  or package manager still reported `match`. `packages` and `display` now
  compare whole sets, `gpu` compares PCI ids where the platform exposes them and
  otherwise compares counts as well as names, and a fabricated extra fails.
- WSL had no verdict table. It now compares what WSL can answer and keeps the
  labelling check, so a correct WSL run is not reported as failing.
- The Linux script lost its non-NVIDIA GPU source and its Wayland path. It again
  uses `lspci` (compared by PCI id) and reads the preferred mode from sysfs —
  the same source the estimate comes from, labelled as such, because comparing
  against `xrandr`'s current mode would manufacture a mismatch.
- `scripts/test_compare.py` adds eleven regression cases for the comparison:
  BOM, malformed input exiting 2 rather than 1, a deselected module that must
  not read as a match, fabricated extras, and supported/unsupported answers.
  CI runs it.
- The release notes claimed the collectors run in CI. What runs is the binary
  and the default modules; no test compares their output, and `display` is never
  run there because it is not a default module. Both documents say that now.

### The published candidate

`v1.0.0-rc.1` was tagged on `6dd54e9` and its release workflow (run 40912501661)
built all five targets and created a draft prerelease. The draft was checked
before publishing: six assets, `SHA256SUMS` verified with `shasum -a 256 -c`,
each archive containing the binary, `LICENSE` and `README.md`, the file types
matching their targets (Mach-O arm64 and x86-64, ELF aarch64 and x86-64, PE32+),
and both macOS binaries reporting `1.0.0-rc.1` — the x86-64 one under Rosetta
emitting a valid document. The release was then published as a prerelease.

One known cosmetic inconsistency: the `README.md` inside those archives is the
one at the tagged commit, which says no tag or release exists yet. It was true of
that commit and is false of the published release. Correcting it means a new
tag; the next release carries the fixed text, and `README.md` on `main` was
updated the same day.

### CI

`d21356e` passed all six jobs on GitHub Actions (run 40911175646): the three
native runners, the MSRV job, the packaging job and the JSON Schema job, which
runs the independent validator and the comparison-script tests.

### Review round on the published candidate

- `scripts/verify-windows.ps1` emitted the GPU observation as a bare list while
  `compare.py` requires `{"names": [...]}`, so every Windows run would have
  stopped at "malformed input". Fixed. The comparer's tests cover the shape it
  accepts, not the shape each producer writes — that is where the two can still
  drift.
- The gate table cited run 40911175646 for the commit that would be tagged. The
  tag is on `6dd54e9`, whose own run 40911728414 was green; both are cited now.
- Three files the CI schema job writes — `full.json`, `partial.json`,
  `broken.json` — were committed in `d21356e` and contain the user name, host
  name and home directory path. They are deleted and ignored now, but they
  remain in the tree of the published tag. Rewriting a published tag would be
  worse than leaving them; the next release does not carry them.
- The checklist said the workflow smoke-tests every packaged binary. Intel macOS
  is built but not executed, which the release notes already said.

### Not verified

Linux, Windows, WSL and Intel hardware, exactly as before. The scripts are how
another tester can close that gap without the maintainer owning the machine; the
release gates in `docs/roadmap.md` say what evidence those runs must produce.

## Repository-hygiene round (2026-10-30)

Phase 16. This round is about what the repository itself carries, not about the
binary.

### The committed run data

Three files — `full.json`, `partial.json`, `broken.json` — were committed in
`d21356e` and were present in the tree of the published tag. Their contents, by
category and without reproducing the values:

| Category | What was there |
| --- | --- |
| Account identity | the OS user name and the machine's host name, as strings |
| Filesystem | the `disk` row's path, which is the same account's home directory |
| Network | one RFC 1918 address on the local interface |
| System description | OS version, hardware model, package counts, and the source strings |

That is the whole of it: no public address, no credential, no serial number, no
file other than the home directory itself. The home path matters because it
names the account a second time, so this is an identity disclosure and not only
a machine fingerprint. The files were produced by running the binary in the
repository root for the schema check and swept in by `git add -A`.

They are deleted and ignored as of `95b8667`. The CI schema job, the native
smoke test and the release smoke test now write their documents to
`$RUNNER_TEMP`, outside the checkout, so a future `git add -A` cannot pick them
up; `.gitignore` keeps the names for anyone reproducing the steps locally.

**Residual risk.** The files remain in the history of `d21356e` through
`74c5506` and in the tree of the `v1.0.0-rc.1` tag, which is public. What is
exposed is an operating-system account name, a host name, a home directory path
and a private address. The OS account name is not the commit author's name or
address, so it is not already public through the commit metadata — this round
previously said otherwise and was wrong.

Removing them means rewriting published history and moving a published tag:
destructive, requires the maintainer's explicit approval, and breaks any clone
or checksum taken against the tag. The decision is to leave them, state the
exposure accurately here, and not carry them forward. The alternative — a
rewrite plus a re-tag — is specified as a separate, approved operation rather
than done quietly.

### Packaging

`cargo package --list` was audited: no run output, no `.json` outside
`docs/json-schema.json` and the hyperfine benchmark exports under
`docs/benchmarks/`, which contain command names and timings only — no user name,
host name or path. The release archives hold the binary, `LICENSE` and
`README.md` and nothing else.

### Diagnostic and strict acceptance

`compare.py` used to report `uncomparable` — and exit 0 — when justfetch said
`unavailable` or `error` while the platform tool had a real answer. For a
release gate that is the wrong direction: a collection failure looked like a
pass.

The verdicts are now `match`, `mismatch`, `unsupported`, `inconclusive`,
`error` and `not covered`, with two modes:

| Mode | Fails on |
| --- | --- |
| Diagnostic (default) | a mismatch or an error |
| Strict (`--strict`) | those, plus any covered module with no comparison |

Exit codes: `0` pass, `1` mismatch or error, `2` bad input or setup, `3` strict
coverage gap. Every `verify-*` script runs strict. On this Mac the run reports
`covered 5/6: 4 match, 1 unsupported` and names `display` as not covered, which
is the honest description of what a comparison against `system_profiler` can
establish there.

Twenty comparison tests cover the counterexamples: an error with native data, an
`unavailable` where the tool answered, `unavailable` with no answer, a missing
module, an extra display, empty observations, a malformed document, the wrong
native shape and fabricated extra adapters or managers. The schema tests are
separate and validate documents, not comparisons.

## Real-hardware round (2026-10-30)

Phase 17. Two real platforms were reachable from this machine; the rest are not,
and stay unverified rather than inferred.

### Platforms

| Platform | Kind | How |
| --- | --- | --- |
| macOS 26.6.2, Mac15,12 (Apple M3) | Physical hardware, bare metal | `scripts/verify-macos.sh` |
| Ubuntu 26.04.1 LTS, kernel 7.0.0-34-generic, aarch64 | **Virtual machine** (QEMU on the same host) | `scripts/verify-linux.sh` against the published `aarch64-unknown-linux-gnu` archive |
| Ubuntu x86-64, macOS, Windows runners | Native CI, not interactive | `cargo test` and the packaged-binary smoke test |
| Windows 11, WSL1/WSL2, Intel macOS | Not reachable | **Unverified.** The scripts are the procedure, not evidence |

Four kinds of evidence appear in this file and they are not interchangeable:

- **Physical hardware**: a real machine, for macOS only so far.
- **Virtual machine**: a real kernel with virtual hardware. Package and locale
  checks are meaningful there; battery, GPU and display are usually absent, so
  those rows exercise the absence paths rather than the collectors.
- **Native CI**: the real binary runs on a real runner and its default modules
  collect, but nothing compares the values with the platform's own tools.
- **Simulated**: fixed documents fed to `compare.py`, and the Windows verifier's
  `-SelfTest`. These test the seam between components and prove nothing about a
  platform.

The Linux run used the **published release artifact**, not a local build: its
`SHA256SUMS` entry verified before it ran. That is the strongest form of this
evidence available here — it tests the binary a user would download.

### macOS run

```
gpu        Apple M3                      {"names": ["Apple M3"]} (system_profiler)  match
battery    84% charging                  {"percent": 84, ...} (pmset)               match
display    1470x956                      —                    not comparable
packages   homebrew formulae 884, ...    {"homebrew formulae": 884, ...} (brew list) match
desktop    unsupported                   —                    unsupported
locale     zh_CN.UTF-8                   zh_CN.UTF-8 ($LANG)                        match
covered 5/6: 4 match, 1 unsupported; declared not comparable: display
```

`desktop` is `unsupported` only because the script independently checked that no
desktop-session variable or Wayland socket exists. `display` is declared not
comparable by the script: justfetch reports the current mode's logical points,
`system_profiler` the panel.

### Ubuntu 26.04 aarch64 run

Session: SSH, `XDG_SESSION_TYPE=tty`, no desktop variables, no
`/sys/class/power_supply` entries, no DRM card, `lspci` lists no display
adapter — a headless machine with no battery, which is exactly the combination
this round was meant to cover.

```
gpu        unsupported                   —                    unsupported
battery    unsupported                   —                    unsupported
display    unsupported                   —                    unsupported
packages   dpkg 675                      {"dpkg": 675} (dpkg-query)  match
desktop    unsupported                   —                    unsupported
locale     C.UTF-8                       C.UTF-8 (LANG)       match
covered 6/6: 2 match, 4 unsupported
```

The four `unsupported` verdicts are backed by independent observations, not by
justfetch's own claim: no DRM card and no PCI display device, no
`power_supply` entry, no connector, no session variables. The two `match`
verdicts are real counts — `dpkg-query` and the `/var/lib/dpkg/status` parse
agree at 675.

### What the real Linux run changed

Two things that no amount of static review would have produced:

1. The GPU and display checks had no way to record a *verified absence*, so a
   headless machine produced a coverage gap instead of a verdict. The script now
   records `absent` when it has looked and found nothing, and `compare.py`
   refuses to count an `unsupported` claim that no observation backs.
2. The same run produced the comparator fixes below. Both were found by running,
   not by reading.

### Comparator changes

- **Duplicate devices.** `display` and `gpu` compared sets, so two identical
  monitors collapsed into one — a document reporting a single display would have
  matched a machine with two. Both compare multisets now (`Counter`).
- **`unsupported` without evidence.** A module marked `unsupported` counted as
  covered even when the script had observed nothing. It now counts only when the
  script lists it in `absent`, having looked and found nothing.
- **Zero valid comparisons.** Strict mode accepted a run where every module was
  unsupported or unobserved. It now requires at least one confirmed match, and
  fails (exit 3) when a module has neither an observation nor a declaration that
  it cannot be compared here.
- **Declared exclusions.** The platform scripts declare modules they cannot
  compare (`not_applicable`) with the reason in the script, so an exclusion is a
  decision on record rather than a silent gap.

31 comparison tests cover these, including two identical displays in both
directions, two identical GPU ids, an `unsupported` claim with and without
backing evidence, a device with no id riding along with one that has one, a
declared `not comparable` module, an undeclared gap, an `unavailable` where the
platform answered, and an all-unsupported run that must never pass.

The suite stands at 183 Rust tests, 31 comparison tests and the Windows verifier
self-test after this round.

### Review round on the real-hardware work

The review reproduced both runs and found two scripts that would have failed on
the machines they exist for:

- `verify-macos.sh` parsed `pmset`'s state as the first word after the
  semicolon. "finishing charge" became `finishing`, which is not a state, so a
  machine on AC that had not yet topped out reported a battery mismatch. The
  phrase is mapped now, and an unrecognised phrase is left out of the comparison
  instead of being forced into a state. The "no battery" test also looked for
  the wrong phrase; `pmset` prints "No batteries available."
- `verify-windows.ps1` declared neither `desktop` nor `locale`, so a correct
  Windows run would have ended at exit 3 with a gap. `desktop` is declared
  absent on evidence (no desktop-session variables), `locale` is declared not
  comparable because justfetch reads the same source a comparison would use.
  The Windows document shape was checked against the comparator with a
  simulated one.
- `verify-wsl.sh` declared `battery` and `display` unconditionally, which the
  new contract forbids: an absence must be a look that came back empty. Every
  declaration now reads sysfs and gives up when a read fails. A missing `/sys/class/drm` is now
  treated as an absence rather than an unreadable path — a WSL guest often has
  no such directory at all — while an unreadable one still declares nothing.
- `verify-linux.sh` could declare the GPU absent when `lspci` had *failed*
  rather than found nothing, and the battery absent without confirming the read
  succeeded. Both require a successful look now. Its battery and display
  evidence also matches what the collector accepts: any supply whose `type` is
  Battery, and "no connector is connected" rather than "no card exists".
- `compare.py` dropped devices with no `id` when comparing by PCI id, so an
  invented device with no id rode along unnoticed. Every device counts now. Its
  verdict is also decided in one place: `evaluate` returns the final verdict
  instead of `main` relabelling a provisional one.

### Finding for v1.1: CPU on an unidentified ARM part

On this VM `cpu` is `unavailable` — "CPU brand not exposed by the system
adapter" — while the machine plainly has four cores. sysinfo 0.39.6 maps an ARM
`CPU part` (0xd03 → Cortex-A53 and so on) to a brand and falls back to
`model name`; this VM reports `CPU part: 0x000`, so neither source yields a name.

Two consequences worth recording:

- It is at least partly a property of this VM: it reports `CPU part: 0x000`,
  which no table can map. Whether real ARM hardware is unaffected is *not*
  established here — sysinfo 0.39.6's part table is finite (its own comment asks
  maintainers to keep extending it), so an unmapped part on real hardware with no
  `model name` would produce the same `unavailable`. Treating this as
  VM-specific is a hypothesis, not a finding; it needs one run on real ARM
  hardware to settle.
- When the brand is unknown, `CpuInfo` carries a required `name`, so the whole
  module becomes `unavailable` and the known core count is discarded. Preserving
  the count would mean making `name` optional in the JSON contract — a v1.1
  change, not one to make at a release candidate. Filed here as a known
  limitation rather than fixed.

### Not covered by this round

Windows 11, WSL1/WSL2, Intel macOS on real hardware, multi-GPU or multi-monitor
machines, a graphical Linux desktop, and remote-desktop sessions. The comparator
handles duplicate devices and the scripts declare what they cannot compare, but
none of those configurations was executed.

## Windows verifier contract round (2026-10-30)

Phase 18. Windows and WSL were not reachable, so this round could not add the
evidence they need. What it could do is remove the possibility that the Windows
verifier and the comparator are each correct and still fail together — which is
exactly the defect the previous round shipped.

### The seam is now tested

`scripts/verify-windows.ps1 -SelfTest` builds the observation document from
fixed values, writes it the same way a real run does, and then:

1. runs `compare.py --strict` on a document that should match, and requires
   exit 0;
2. reads `native.json` back and requires the `absent` and `not_applicable`
   lists, with their contents, to survive serialization;
3. calls the real collector function, serializes its output, and requires every
   key to survive and the comparator not to reject it as malformed;
4. mutates the document to disagree and requires exit 1.

It needs no Windows API, so CI runs it on every push. Verified here that it has
teeth, by breaking it in each way a later edit could: deleting the `absent` line
from the collector, deleting the `not_applicable` line, returning a scalar
instead of a list, truncating the JSON with `-Depth 1`, renaming a module key,
and making `compare.py` always exit 0. Each makes the step exit 1; restoring
returns exit 0.

Two of the cases go further than structure, because the review found the first
version stopped short:

- A **stock Windows** document — an adapter and a monitor, no battery, no Scoop,
  no desktop — must pass strict. Without it the script could keep exiting 3 on a
  machine with nothing wrong except a package manager it does not use.
- `JF_SCOOP_APPS` drives the collector's own Scoop branch, so "declare it absent
  when the directory is missing" and "count two apps when there are two" are
  tested on any host. Deleting the `absent += 'packages'` line makes the
  self-test exit 1.
- An undeclared gap must exit 3, which is the strict-only code the script
  promises and otherwise never reaches.
- Scoop installed with an empty `apps` directory: justfetch reports
  `unsupported`, so the verifier declares the absence rather than reporting a
  count of zero and failing as a mismatch.

Not covered by the self-test: the battery branch, because `Win32_Battery` only
exists on Windows. `JF_SCOOP_APPS` has no equivalent for CIM, so a machine with
and without a battery can only be distinguished on a Windows host.

The previous round's defect was precisely a list that was computed and never
written. A later round can reintroduce it, but not without this step failing.

### What the self-test does not prove

Nothing about Windows itself. `Get-CimInstance`, `Screen.AllScreens` and
`SetProcessDPIAware` are never called on a CI runner, so their availability,
their return shapes and the multi-monitor behaviour remain unverified until
someone runs the script on Windows. The self-test proves only that whatever
those calls return will be written in a form the comparator accepts.

### Display: adapters, monitors, resolution and refresh

Three different quantities were easy to confuse, so each is named:

| Quantity | justfetch | Verifier |
| --- | --- | --- |
| Adapter | `EnumDisplayDevicesW`, one entry per adapter | `Win32_VideoController`, one per adapter — compared by name, and a mismatch is a wording difference more often than a defect |
| Monitor | `EnumDisplayMonitors` + `GetMonitorInfoW`, one entry per monitor | `Screen.AllScreens`, one per monitor — same unit |
| Resolution | `rcMonitor` width and height | `Screen.Bounds` after `SetProcessDPIAware` |
| Refresh rate | `EnumDisplaySettingsW` per monitor device | **not compared**: `Screen` exposes no refresh rate, and the script does not invent one |

Under display scaling the resolutions may differ even on a correct machine:
justfetch has no DPI-awareness manifest, so `GetMonitorInfoW` returns
virtualized coordinates for it, while the verifier calls `SetProcessDPIAware`
first. A resolution mismatch on a scaled display is therefore a limitation of
the comparison, not necessarily a defect. A GPU name mismatch has a similar
cause — `Win32_VideoController` and `EnumDisplayDevicesW` word the same adapter
differently — so either should be reviewed in diagnostic mode before being
filed.

### Release workflow: candidate and stable are derived, not hardcoded

`--draft` stays in both cases; publishing remains a maintainer action. The
`--prerelease` flag is now derived from the version — a `-rc.N` version is
marked prerelease, a final one is not — so a stable version cannot be published
as a candidate by forgetting to edit the workflow. `scripts/check-release.sh`
checks both arms of that rule and reports which mode the tree is in; changing
the stable arm to `--prerelease` makes it fail, which is how the guard was
tested. A version bump to `1.0.0` without `docs/release-notes/v1.0.0.md` fails
the check immediately.

### CPU on an unidentified ARM part: a pinned regression

`cpu_detection` was extracted from the collection path so the decision can be tested
without a machine that reproduces it. Three cases are pinned: an empty brand
with a known core count is `unavailable` and the count is discarded; a named
CPU reports both fields; a failed core count is an `error`, not an absence.

The first is the v1.0 behaviour, recorded as such. Keeping the core count when
the name is unknown means making `CpuInfo.name` optional in the JSON contract —
a v1.1 schema decision. The test exists so that change happens deliberately.

### Source equivalence with the published candidate

The CPU change touched product code after `v1.0.0-rc.1` was published, so the
question is whether the binary a user would get from this tree still behaves the
same. `scripts/compare-builds.sh` answers it without network access:

```sh
gh release download v1.0.0-rc.1 --pattern "*aarch64-apple-darwin.tar.gz" --dir /tmp/rc
cargo build --release --locked
scripts/compare-builds.sh /tmp/rc/*.tar.gz
```

It compares every field except the five that legitimately change between two
runs on the same machine — `uptime`, `memory`, `swap`, `battery`, `disk` — and
fails if any other differs. Run on 2026-10-30 the output was:

```
compared: cpu, desktop, display, environment, gpu, host, identity, kernel,
          local_ip, locale, os, packages, schema_version, shell, terminal
excluded as volatile: battery, disk, memory, swap, uptime
identical on every non-volatile field
```

`cpu` is inside the compared set, so the refactor is shown not to have changed
it. This covers macOS only; Linux was not re-run because the virtual machine was
removed after Phase 17.
