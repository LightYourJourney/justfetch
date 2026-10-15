# Performance

What justfetch costs to run, measured on one machine, with the raw evidence committed next to this file. These are measurements, not targets. No figure from another project's documentation is quoted as a result of ours.

The short version: on this machine under interactive load, **wall-clock time is not a reproducible measurement** — it swings by more than 2x between rounds. Instruction counts are, to within about 2%. The current baseline is therefore reported in instructions and CPU time first, with wall clock shown only as a load-contaminated upper bound.

## Current baseline — 2026-10-16

Apple M3 (8 logical cores), 16 GiB, macOS 26.6.2 / Darwin 25.6.0 arm64. Homebrew rustc/cargo 1.99.0. Build: `cargo build --release --locked`, thin LTO, one codegen unit, stripped, no extra `RUSTFLAGS`. Hyperfine 2.0.0. Release binary 798,336 bytes.

Protocol: ten rounds of ten runs per command, warmup one per command per round. Hyperfine runs one command's samples consecutively, so a load spike during a round biases that whole command; rotating the command order every round (forward on odd rounds, reversed on even ones) spreads any drift across all seven instead of concentrating it. `NO_COLOR=1` for the whole suite, `--shell none --output pipe`. 700 samples, every one exited zero.

The machine was **not** quiesced, at the request of the person running it. Load average was 7.4 when the run began and 36–41 during it; the benchmark's own process churn is part of that figure.

| Command | Wall mean | Wall median | Wall min | Wall σ | CPU median | Peak RSS | **Instructions** | Instr. CV |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| justfetch --no-color | 14.3 ms | 10.1 ms | 5.4 ms | 8.5 ms | 7.0 ms | 6.2 MiB | **27.8 M** | 1.2% |
| justfetch --no-color --logo none | 20.6 ms | 11.1 ms | 5.3 ms | 24.3 ms | 6.9 ms | 6.2 MiB | **27.8 M** | 1.9% |
| justfetch --format json | 18.3 ms | 12.6 ms | 5.3 ms | 14.4 ms | 7.2 ms | 6.1 MiB | **27.7 M** | 1.4% |
| justfetch --help | 18.2 ms | 12.7 ms | 5.4 ms | 17.9 ms | 7.0 ms | 6.1 MiB | **27.4 M** | 1.7% |
| Fastfetch 2.69.0, seven modules | 32.1 ms | 19.6 ms | 9.2 ms | 23.0 ms | 11.1 ms | 9.3 MiB | 41.8 M | 1.8% |
| pfetch-rs 3.0.2, seven modules | 32.8 ms | 16.8 ms | 7.5 ms | 37.5 ms | 8.6 ms | 8.8 MiB | 34.2 M | 1.2% |
| Macchina 6.4.0, seven readouts | 82.3 ms | 39.8 ms | 14.4 ms | 102.9 ms | 17.7 ms | 10.2 MiB | 46.0 M | 2.1% |

Read the instructions column as the cost of the work and the wall column as an upper bound that includes waiting for a busy scheduler. Relative to a full justfetch run, pfetch-rs executes 1.23x the instructions, Fastfetch 1.51x and Macchina 1.66x. Peak RSS is 6.1–6.2 MiB against 8.8–10.2 MiB for the other three.

One result is worth stating plainly because it shapes where further work should go: **`--help` costs 27.4 M instructions and a full run costs 27.8 M.** Collection and rendering together are about 1.5% of what the process spends. The remaining 98.5% is process creation, dynamic linking and argument parsing, which no change to a detector will move. `--help` is a launch baseline, not a pure process-creation measurement — it still builds the clap parser — but the ordering is unambiguous.

## Why wall clock was demoted

Two earlier rounds disagreed by roughly 3x on wall time (10.4 ms on 2026-10-10, 3.2 ms on 2026-10-11) while peak RSS matched to 0.05 MiB. The note added at the time guessed a per-run harness cost. That guess was **wrong**, and the 2026-10-16 round measured it rather than inferring it.

Same four commands, sandboxed and unsandboxed, 50 runs each, minutes apart:

| Command | Sandboxed | Unsandboxed |
| --- | ---: | ---: |
| `/usr/bin/true` | 2.8 ± 0.6 ms | 2.3 ± 0.7 ms |
| justfetch --help | 8.5 ± 1.7 ms | 9.4 ± 4.1 ms |
| justfetch --no-color | 7.4 ± 1.1 ms | 17.4 ± 12.4 ms |
| Fastfetch | 16.1 ± 7.3 ms | 52.5 ± 32.4 ms |

The sandbox is not the cause; the unsandboxed run was slower and far noisier, because the machine was busier by then. What separates the metrics is the spread, measured across the whole 2026-10-16 run: wall-clock coefficient of variation was **59% to 125%** depending on the command, CPU time 17% to 21%, and instructions **1.2% to 2.1%**. Wall clock is measuring the machine's mood; instructions are measuring justfetch.

## What this does and does not show

- **Comparability.** Each competitor was checked to produce the seven selected fields before measuring, and no competitor output is consumed by justfetch. They are not identical work. Fastfetch reports a friendly device name and CPU frequency where justfetch reports a raw model identifier and a logical count, and memory accounting differs across all four, particularly pfetch-rs. Macchina 6.4.0 draws ASCII art and positions its output with absolute cursor addressing (`ESC[1;1H`). No command-line flag disables either. A theme file can hide the art — the binary carries `hide_ascii` and `prefer_small_ascii` theme keys — but `--help` does not document the theme schema, `-t` silently falls back to the default on an unrecognised name, `XDG_CONFIG_HOME` is ignored on macOS, and the only theme directory it will read is the user's real `~/.config/macchina/themes`. Writing into someone's configuration to make a benchmark look better is not a trade worth making, so this round measured Macchina as it ships: its figures include rendering work the other three were explicitly told to skip, and are an upper bound on comparable Macchina work rather than a like-for-like ratio.
- **Shell detection during the run.** Hyperfine is the parent process, so the immediate-parent lookup sees hyperfine rather than a shell and justfetch falls back to its labelled configured default. That is a fixed cost in every sample, not a variable one.
- **What the numbers are not.** They are not a speedup claim, a ranking that will hold on other hardware, or evidence about Linux and Windows. Wall painting in a real terminal is not measured. Peak RSS is the process peak-resident metric, not retained heap or allocation count. No cold-cache or CPU-pinning setup was used.
- **Still missing.** A quiesced-machine round. The person running this machine declined to stop other work, so the wall figures above stay load-contaminated; the instructions figures do not depend on that and are the ones to compare.

## Reproduce

Needs Hyperfine (2.0.0 was used), Fastfetch, pfetch-rs and Macchina on `PATH`. This is the script that produced the artifact above:

```sh
cargo build --release --locked
export NO_COLOR=1
out=$(mktemp -d)
cmds='target/release/justfetch --no-color
target/release/justfetch --no-color --logo none
target/release/justfetch --format json
target/release/justfetch --help
fastfetch --config none --structure OS:Host:Kernel:Uptime:CPU:Memory:Shell --logo none --pipe true --detect-version false
pfetch --no-color --logo none --info "os host kernel uptime cpu memory shell"
macchina -o operating-system,machine,kernel,uptime,processor,memory,shell'
i=1
while [ "$i" -le 10 ]; do
  if [ $((i % 2)) -eq 1 ]; then order=$(printf '%s\n' "$cmds"); else order=$(printf '%s\n' "$cmds" | tail -r); fi
  n=1
  printf '%s\n' "$order" | while IFS= read -r c; do
    hyperfine --warmup 1 --runs 10 --shell none --output pipe --export-json "$out/r$i-c$n.json" "$c"
    n=$((n + 1))
  done
  i=$((i + 1))
done
```

`tail -r` is the BSD spelling; GNU systems spell it `tac`. On this loaded machine the ten rounds took about a minute and a half.

Raw output for every sample is in [benchmarks/macos-arm64-interleaved.json](benchmarks/macos-arm64-interleaved.json): the full command strings, all 700 measurements with wall clock, CPU time, user and system time, peak resident bytes, CPU cycles, instruction counts and exit codes, plus the load samples and tool versions. No statistic is baked into that file.

## Superseded rounds

Kept because they are the reason the method changed, and because deleting an inconvenient measurement is worse than showing a noisy one.

**2026-10-10.** Apple M3, same machine, Hyperfine 2.0.0, 10 warmups and 100 runs per command, binary 798,288 bytes. Raw data in [benchmarks/macos-arm64.json](benchmarks/macos-arm64.json), all 600 runs exiting zero.

| Command | Mean ± σ | Mean peak RSS |
| --- | ---: | ---: |
| justfetch --no-color | 10.428 ± 1.383 ms | 6.172 MiB |
| justfetch --no-color --logo none | 10.397 ± 1.367 ms | 6.175 MiB |
| justfetch --format json | 9.798 ± 1.450 ms | 6.163 MiB |
| justfetch --help | 9.863 ± 1.432 ms | 6.093 MiB |
| Fastfetch 2.69.0 | 13.283 ± 3.391 ms | 9.409 MiB |
| pfetch-rs 3.0.2 | 9.334 ± 2.098 ms | 8.799 MiB |

**2026-10-11.** A 40-run re-run of the same commands on the same binary produced roughly one third of those wall times across the board, including `--help`, with RSS agreeing to 0.05 MiB. The ranking also reversed: pfetch-rs was faster than justfetch on 2026-10-10 and slower on 2026-10-11. A third attempt during a busy period (load 11.6 / 29.7 / 31.7, `/usr/bin/true` baseline 31.5 ± 26.9 ms) was discarded as unusable.

Both rounds' wall times should be treated as upper bounds measured under unidentified load. Neither should be quoted as a baseline, and no comparison against Fastfetch, pfetch-rs or Macchina should be taken from them.
