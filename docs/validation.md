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
- Dependency MSRVs in `docs/research.md` match the registry manifests: sysinfo 0.39.6 declares 1.95, clap 4.6.7 declares 1.85, serde 1.0.229 declares 1.56, serde_json 1.0.151 declares 1.71, unicode-width 0.2.2 declares 1.66, unicode-segmentation 1.13.3 declares 1.85, anstyle 1.0.14 declares 1.66, libc 0.2.190 declares 1.65, windows-sys 0.61.2 declares 1.71, toml 1.1.8 and its chain declare 1.85.

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

The benchmark at that point had no Macchina measurement and no reproducible wall-time baseline, so the spec's "benchmark against Fastfetch, Macchina and pfetch-rs" was only partly satisfied. Both were recorded in performance.md rather than papered over. **Closed on 2026-10-16; see the benchmark section below.**

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

**A hypothesis was tested and failed.** The note written on 2026-10-11 guessed that a uniform per-run harness cost explained the 3x wall-time drift. Measuring the same four commands sandboxed and unsandboxed minutes apart showed the sandbox is not the cause — the unsandboxed run was slower and far noisier because the machine had become busier. Host load explains it; the earlier guess was wrong and `docs/performance.md` now says so.

**A finding worth acting on later.** `justfetch --help` costs 27.4 M instructions and a full run 27.8 M. Collection and rendering are about 1.5% of the process; the rest is process creation, dynamic linking and argument parsing. No detector change can move the remaining 98.5%, so future performance work that matters would have to target startup, not collection.

**Corrections made during this round.** Two claims of mine were wrong and were fixed before commit. The first, in `docs/performance.md`, was the harness-cost inference above. The second was a sentence asserting Macchina "has no such key in its binary" for hiding ASCII art: a wider search of the binary found `hide_ascii` and `prefer_small_ascii` theme keys, so the sentence was replaced with what is actually true — no CLI flag exists, the theme schema is undocumented, `-t` silently falls back on an unknown name, and `XDG_CONFIG_HOME` is ignored on macOS, so using a theme would mean writing into the reader's own config directory. Macchina was measured as it ships and its figures are labelled an upper bound rather than a like-for-like ratio.

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

**Selection-driven collection.** The claim that an unselected module is never queried was measured rather than asserted: 100 runs each with `hyperfine --shell none --output pipe`, comparing a full run against progressively smaller selections. Instruction medians: full run 28.47 M, `--modules cpu` 27.81 M (97.7%), `--modules cpu,memory` 27.91 M (98.0%), every module except `host` 28.34 M (99.6%). Raw measurements are in [benchmarks/module-selection-2026-10-27.json](benchmarks/module-selection-2026-10-27.json). The saving is real and small because process startup dominates; no wall-clock figure is quoted, for the reason recorded in performance.md.

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

**Rejected finding.** Both reviewers noted that `--format json` still reads and validates `[theme] file` and `layout.logo_file`, so a missing presentation file fails a JSON run. That is deliberate and now documented: the configuration is validated once, the same way for every output format, and only the document's contents ignore presentation. Making validation conditional on the format would give the same file two different meanings, and a user whose configuration names a missing file has a broken configuration whether or not that run draws it.

**Not verified.** No Windows or Linux terminal was available interactively. The colour-depth checks above ran on macOS only; the platform-specific ANSI policy is exercised by CI and by cross-target compilation, which is not the same thing. Font-dependent rendering of `●` and of bold is not asserted anywhere, and no screenshot comparison was performed.

**Two-axis review of this round.** A Standards review and a Spec review ran independently against the same working tree. Both axes reported; the findings are closed:

- `[color] accent` did not override a theme that names its title or labels; it only filled a gap. The documentation called it an override, so the code was wrong. Verified in a terminal: `--theme nord` and `--theme nord` with `accent = "red"` now emit different title bytes, where before they were identical.
- `justfetch themes` showed only the palette, so a theme could not actually be previewed. The name is now drawn in the theme's title colour and a label/value/rule sample follows the palette.
- Explicitly chosen artwork was silently dropped on a narrow terminal. A named logo now moves above the information instead of disappearing; automatic artwork still gives way, as before.
- Five `expect()` calls sat on production paths, one of them in `Config::default`, which runs on every invocation. All are gone; the two colour lookups that were "impossible to fail" are computed without assuming it.
- The distribution table was written twice — once for detection, once for explicit selection — so a new distribution meant editing both. It is now one table with the two alias sets layered on top.
- Theme and logo file loading was duplicated between the configuration path and the command line; both now call one loader.
- The theme listing styled its chips with a local copy of the renderer's colour helper, and panicked on a missing theme while the JSON listing silently skipped it. Both now share one helper and skip the same way.
- Documents still described the previous state: `architecture.md` said the configuration file was the only file read and listed themes as roadmap, `AGENTS.md` and `CONTRIBUTING.md` repeated the scope error, `architecture.md` said there were two subcommands, and the configuration reference omitted `--theme`/`--layout` from the precedence list and described `color.accent` as ANSI names only.

**Rejected finding.** A Standards finding claimed the default output was not byte-identical because the swatch row was new. It is not new: `e4317a7`'s pinned snapshot contains the same `o o o o o o o o` row, and the field-by-field comparison above shows the text is unchanged. The claim in this file was corrected only where it was genuinely inaccurate — colour escape placement, not the text.

**Accepted deviation.** `--theme-file`, `--logo-file` and `--format json themes` go beyond the specification's suggested CLI list. The file flags are how a custom theme or logo is selected without a configuration file, which the spec requires; the JSON listing exists so that asking for JSON yields JSON.

**Test count:** 132, up from 87.
