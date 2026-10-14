# Performance baseline

Measured 2026-10-10 on Apple M3 (8 logical CPUs), Mac15,12, 16 GiB RAM, macOS 26.6.2 / Darwin 25.6.0 ARM64. Toolchain: Homebrew rustc/cargo 1.99.0. Build: `cargo build --release --locked`, thin LTO, one codegen unit, stripped; no extra RUSTFLAGS. Hyperfine 2.0.0. Measurements ran outside the development tool sandbox as an ordinary user so native uptime calls were permitted. Other desktop activity was not stopped.

| Command | Mean ± sample standard deviation | Mean peak RSS |
| --- | ---: | ---: |
| justfetch --no-color | 10.428 ± 1.383 ms | 6.172 MiB |
| justfetch --no-color --logo none | 10.397 ± 1.367 ms | 6.175 MiB |
| justfetch --format json | 9.798 ± 1.450 ms | 6.163 MiB |
| justfetch --help | 9.863 ± 1.432 ms | 6.093 MiB |
| Fastfetch 2.69.0, seven modules, no logo/colors/version probes | 13.283 ± 3.391 ms | 9.409 MiB |
| pfetch-rs 3.0.2, seven modules, no logo/colors | 9.334 ± 2.098 ms | 8.799 MiB |

Binary at measurement: **798,288 bytes** (779.6 KiB), ARM64 Mach-O. Raw measurements, RSS, exit codes and timestamps are in [benchmarks/macos-arm64.json](benchmarks/macos-arm64.json), Hyperfine JSON schema 2. All 600 recorded runs exited zero. Each command had 10 warmups and 100 measured runs. Output was consumed through a pipe, without a shell, so this includes launch, collection, formatting, writes and exit; it does not measure terminal painting. `--help` avoids collection and provides a launch/CLI baseline, not a pure process-creation measurement.

Reproduce from the repository root (requires already installed Hyperfine/Fastfetch/pfetch-rs):

```sh
cargo build --release --locked
hyperfine --warmup 10 --runs 100 --shell none --output pipe \
  --export-json docs/benchmarks/macos-arm64.json \
  'target/release/justfetch --no-color' \
  'target/release/justfetch --no-color --logo none' \
  'target/release/justfetch --format json' \
  'target/release/justfetch --help' \
  'fastfetch --config none --structure OS:Host:Kernel:Uptime:CPU:Memory:Shell --logo none --pipe true --detect-version false' \
  "pfetch --no-color --logo none --info 'os host kernel uptime cpu memory shell'"
```

## Comparability and limits

The competitor commands were checked to produce the seven selected fields before measuring; no competitor stdout was used by justfetch. Fastfetch host includes a friendly device name and CPU details/frequency, while justfetch reports a raw hardware model identifier and logical count; memory accounting differs, particularly pfetch-rs. justfetch text retains identity/dividers/palette even with logo none. Hyperfine is the parent, so immediate-parent shell detection may use the labeled configured-default fallback. No artificial cold-cache setup was used. Macchina is not installed and was not measured; no automatic installation was performed.

## Independent re-measurement (2026-10-11)

The table above was re-run once more on the same machine, same Release binary (798,288 bytes, byte-identical size) and same commands. It did not reproduce the wall times, and the discrepancy is environmental rather than a code change.

| Command | 2026-10-10 mean (100 runs) | 2026-10-11 mean (40 runs) | Peak RSS, both rounds |
| --- | ---: | ---: | --- |
| justfetch --no-color | 10.428 ms | 3.2 ± 0.2 ms | 6.172 / 6.2 MiB |
| justfetch --logo none | 10.397 ms | 3.0 ± 0.3 ms | 6.175 / 6.2 MiB |
| justfetch --format json | 9.798 ms | 3.4 ± 0.6 ms | 6.163 / 6.2 MiB |
| justfetch --help | 9.863 ms | 3.0 ± 0.2 ms | 6.093 / 6.1 MiB |
| Fastfetch 2.69.0 | 13.283 ms | 4.8 ± 0.3 ms | 9.409 / 9.4 MiB |
| pfetch-rs 3.0.2 | 9.334 ms | 4.0 ± 0.3 ms | 8.799 / 8.8 MiB |

Peak RSS agrees to within reporting granularity while wall time differs by roughly 3x for every command, including `--help`, which performs no collection. A per-run harness cost that applies uniformly is the likely cause, but the original run's exact process environment was not recorded, so this is an inference rather than a measurement. A further attempt during a busy period (load average 11.6 / 29.7 / 31.7, with an unrelated `true` baseline measuring 31.5 ± 26.9 ms) produced unusable variance and was discarded.

Two consequences for reading the table above. First, treat the 2026-10-10 wall times as an upper bound taken under load, not as a stable baseline: the machine was not quiesced in either round and the documented σ values are 13–30% of their means. Second, the relative ranking is **not** stable — the 2026-10-10 round had pfetch-rs faster than justfetch, the 2026-10-11 round had justfetch faster than pfetch-rs. No comparison against Fastfetch or pfetch-rs should be quoted from either round.

Peak RSS is the stable part of this record: it reproduces to within 0.05 MiB across rounds and is unaffected by CPU contention. A publishable wall-time baseline still requires a quiesced machine and a recorded process environment; that is a v1.0 release task, not a v0.1 claim.

Variance is material at these durations, and help costs nearly as much as a full invocation. These results do not establish stable relative rankings across systems, isolate individual collector costs, or prove Linux/Windows performance. No speedup target or marketing claim is made. RSS is Hyperfine's process peak-resident metric, not retained heap or allocation count. Cross-platform native APIs, correctness and maintainability take priority over reducing the remaining startup floor. Per-module profiling remains a future diagnostic feature.
