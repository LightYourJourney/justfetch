# Compatibility and data semantics

The support contract is conservative: a platform adapter is implemented, but only explicitly recorded environments have been exercised. No administrator access, external commands or runtime network are required.

## Evidence matrix

| Platform | Implementation | Local compile check | Actual detector run | Native CI |
| --- | --- | --- | --- | --- |
| macOS ARM64 | Complete | Native build/test/release | Apple M3 / Mac15,12 / macOS 26.6.2 locally; Apple M1 (Virtual) / VirtualMac2,1 on the macOS runner. Seven fields available in both | Executed: run 38604699769 |
| macOS Intel | Complete | Passed cargo check (Rust 1.98.1; no cross link/run) | Not tested on Intel hardware | macOS runner is ARM64; no Intel runner |
| Linux ARM64 | Complete | Passed cargo check (Rust 1.98.1; no cross link/run) | Not tested on ARM64 hardware | Ubuntu runner is x86-64 |
| Linux x86-64 | Complete | Passed cargo check (Rust 1.98.1; no link/run) | Executed on Ubuntu 24.04 (Azure VM, kernel 6.17.0-1022-azure): all seven fields available, os-release and proc parsers live, parent shell resolved to `bash` as `current_parent` | Executed: run 38604699769 |
| Windows x86-64 | Complete | GNU-target cargo check (Rust 1.98.1; no link/run) | Executed on Windows Server 2025 Datacenter build 26100, MSVC toolchain: all seven fields available, registry OS name, GlobalMemoryStatusEx and ToolHelp parent snapshot live | Executed: run 38604699769 |
| WSL1 | Linux adapter, WSL detection/semantic safeguards | Pure parser fixtures only | Not tested in WSL1 | No WSL runner |
| WSL2 | Linux adapter, explicit WSL2 marker detection | Pure parser fixtures only | Not tested in WSL2 | No WSL runner |
| BSD and other OS | Out of v0.1 scope | Compile-time unsupported | Not tested | None |

"Executed" means the release binary ran on a real kernel of that platform and returned the JSON above; it does not mean every distribution, device or hardware configuration has been tried. What the runners establish is that the Linux `os-release`/`proc`/sysfs and parent-shell paths and the Windows registry/memory/ToolHelp paths work on at least one real system each. Arch, Debian, Fedora, Gentoo, openSUSE, NixOS, Intel Macs, ARM64 Linux, WSL and container guests remain unexercised.

One behaviour the Windows runner made concrete: `cargo run` is the parent there, because Cargo replaces itself with `exec` on Unix but must spawn a child on Windows. Shell detection therefore resolved to `bash` as `current_parent` on Linux and fell back to `cmd` as `configured_default` on Windows, which is the documented immediate-parent-only semantics working as intended rather than a platform defect.

## Per-module sources

| Field | Linux / WSL | macOS | Windows |
| --- | --- | --- | --- |
| OS | /etc/os-release, then /usr/lib/os-release only if absent | kern.osproductversion; no stale marketing-name map | sysinfo native Windows product/build registry |
| Host | DMI product_name or device-tree model; hostname fallback explicitly labeled | hw.model hardware identifier, e.g. Mac15,12 | Hostname explicitly labeled, no WMI/model guess |
| Kernel | uname name/release | uname Darwin release | Windows NT build from CurrentBuildNumber |
| Uptime | /proc/uptime raw seconds | kern.boottime and current wall clock, checked for inconsistent timestamps | GetTickCount64 seconds |
| CPU | sysinfo kernel/native CPU brand, logical processor count | sysinfo exact CPU brand; Apple Silicon/Intel, no M-series guess | sysinfo native CPU brand; GetActiveProcessorCount across all processor groups |
| Memory | /proc/meminfo; KiB converted to bytes | hw.memsize and Mach host_statistics64 | GlobalMemoryStatusEx |
| Shell | getppid + /proc/parent/exe; SHELL default fallback | getppid + proc_pidpath; SHELL default fallback | ToolHelp parent ID + QueryFullProcessImageNameW; ComSpec default fallback |

Host is the only field with an internal fallback chain, and its exact behavior differs by platform and failure kind. On Linux an absent `/sys/devices/virtual/dmi/id/product_name` (and absent device-tree model) falls through to the labeled hostname, as does a placeholder string such as `System Product Name`; if the file exists but cannot be read, is not UTF-8, or exceeds 64 KiB, the read error is reported for Host instead of being downgraded to a hostname. On macOS `hw.model` is the sole source and there is no hostname fallback: an unreadable, empty or non-UTF-8 value is reported as an error. On Windows Host is always the labeled hostname and no WMI or model query is attempted. The fallback never invents a model, and an error is never rewritten as a successful hostname.

Every field has available/unavailable/unsupported/error status. No v0.1 collector currently emits `unsupported`: the state exists in the schema, is reachable in JSON and renders as `Unsupported`, but every field this version collects is either available or fails with a stated reason and location. A future collector that skips a platform outright — a GPU module on a host without one — is what would populate it. Available fields have source/confidence and optional note. Some sysinfo APIs report only Option, so unavailable can include library-level access/detection failure; justfetch cannot recover an error the dependency does not expose. Direct native calls and file reads preserve errors. A field failure does not suppress other results or make JSON invalid. Exit 0 means collection/rendering completed; inspect field status to assess completeness. Invalid CLI usage and output-write failures return nonzero; broken pipes exit quietly.

## Exact memory definitions

- Linux: `used = MemTotal - MemAvailable`. /proc labels these quantities kB, meaning 1024 bytes. When MemAvailable is absent, estimate available as MemFree + Buffers + Cached + SReclaimable - Shmem (optional last two default to zero). That fallback is labeled estimated. Missing/invalid counters and arithmetic overflow are errors; no fabricated zero result.
- macOS: total is hw.memsize. Used bytes are `(internal_page_count - purgeable_count + wire_count + compressor_page_count) * page_size`: app, wired and compressed physical memory. This is not a process-RSS sum, memory pressure metric or total-minus-free estimate.
- Windows: total physical RAM minus available physical RAM reported by GlobalMemoryStatusEx. Swap/commit/pagefile are excluded.

These OS accounting definitions differ. JSON exposes `definition` and `scope`; cross-OS percentages are not identical workload metrics. Terminal values use binary GiB (2^30 bytes).

## WSL and virtualized/container environments

The displayed OS is the current Linux distribution. uname identifies the running kernel interface. An explicit WSL2/microsoft-standard kernel marker selects WSL2; Microsoft markers or WSL environment hints without that evidence produce WSL with an unknown version, never a guessed WSL1. Custom kernels and removed environment hints may make detection impossible. WSL memory and uptime have wsl_environment scope: WSL1 reports come from its compatibility layer and WSL2 from its managed kernel/VM. Neither is a claim about physical Windows host RAM or host boot time. DMI/device-tree hardware probing is skipped under WSL; Host is the Linux environment hostname.

On ordinary VMs, hardware identifiers describe the guest. Container kernel, uptime and physical-memory counters can be shared with the host; v0.1 does not implement cgroup limits or a container identity module. These reports are kernel-visible system totals, not container quotas.

## Terminal behavior

ASCII logos need no special font. Display widths use Unicode tables and grapheme segmentation; terminals may differ for ambiguous-width glyphs or emoji. The terminal width is queried without altering state and falls back to 80 columns. The logo is drawn only when the width is at least the logo width plus three separator columns plus 36 columns for the information field; below that the layout becomes single-column with no logo. Long fields wrap on grapheme boundaries, but a single grapheme wider than the columns left for it cannot be represented at all: it is replaced by `?`, so a terminal narrower than one CJK character or emoji elides that character rather than overflowing the line. C/POSIX or non-UTF-8 locales and TERM=dumb use ASCII presentation, replacing non-ASCII system characters with `?`; JSON preserves originals. Explicit no-color, non-TTY, nonempty NO_COLOR and TERM=dumb disable ANSI. Windows colors require virtual-terminal output already enabled; justfetch does not change console modes.

Untrusted ESC/CSI/OSC/DCS strings and control/bidi characters are removed before terminal layout. JSON escapes control characters but retains original values; downstream clients must sanitize before displaying JSON values in terminals. User/host identity can be sensitive and is visible by default. Username is an environment hint, marked estimated. A configured-default shell is labeled `~ name (default)` and is not evidence of the currently interactive shell.

## Known validation limits

WSL, Intel Macs, ARM64 Linux, containers, and every distribution other than Ubuntu remain unexercised, as does any path that depends on particular hardware. The matrix above records exactly what was run and where; where a row says a check ran, it ran on that one configuration and nothing wider. Cross-compilation checks prove compilation only, not linking, terminal appearance or system-call behavior. See validation.md for exact commands and results.
