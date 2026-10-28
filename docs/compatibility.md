# Compatibility and data semantics

The support contract is conservative: a platform adapter is implemented, but only explicitly recorded environments have been exercised. No administrator access, external commands or runtime network are required.

## What each state means

Every field carries one of six states. The distinction decides whether "no
value" means "not here", "not read" or "failed":

| State | Meaning | Example |
| --- | --- | --- |
| `available` | Measured, with a source and a confidence | A battery reading |
| `unavailable` | The platform should be able to answer but did not | `/sys` could not be read |
| `unsupported` | The question does not apply here | No battery, no display, Windows swap, macOS desktop |
| `redacted` | Withheld because `--privacy` was passed | User name |
| `deselected` | Not collected because the module was not selected | Any module outside `--modules` |
| `error` | An API was called and reported a failure | `CGGetActiveDisplayList` returning an error code |

A machine with no battery, no display or no graphics adapter reports
`unsupported` with a reason rather than a zero, an empty list or a guess — where
the platform can prove the absence (`/sys` lists no connector, Windows lists no
monitor, macOS reports no internal battery). When an API simply returns nothing
and a sandbox that denies it would look identical, the state is `unavailable`
with a reason saying so; macOS display is reported that way.

## Evidence matrix

| Platform | Implementation | Local compile check | Actual detector run | Native CI |
| --- | --- | --- | --- | --- |
| macOS ARM64 | Complete | Native build/test/release | Apple M3 / Mac15,12 / macOS 26.6.2 locally; Apple M1 (Virtual) / VirtualMac2,1 on the macOS runner. All fields available at the time of that run; `battery` was added later and verified locally (below) | Executed: run 40301536939 |
| macOS Intel | Complete | Cross-compiled locally and executed under Rosetta 2 on Apple Silicon; not run on Intel hardware | Not tested on Intel hardware | macOS runner is ARM64; no Intel runner |
| Linux ARM64 | Complete | Passed cargo check (Rust 1.98.1; no cross link/run) | Executed in the release workflow: the packaged archive ran on an ARM64 runner and reported `os` and `memory` available. Not exercised interactively, and no distribution was tested | Release workflow run 40304542365 |
| Linux x86-64 | Complete | Passed cargo check (Rust 1.98.1; no link/run) | Executed on Ubuntu 24.04 (Azure VM, kernel 6.17.0-1022-azure), then re-run in CI with the eleven-field build: all eleven fields then available, os-release and proc parsers live, parent-shell path exercised. `battery` added later and not yet run on Linux | Executed: run 38604699769, then 40301536939 |
| Windows x86-64 | Complete | GNU-target cargo check (Rust 1.98.1; no link/run) | Executed on Windows Server 2025 Datacenter build 26100, MSVC toolchain, then re-run in CI with the eleven-field build: nine fields available, `swap` unsupported, and `terminal` unavailable because a CI job has no terminal environment; registry OS name, GlobalMemoryStatusEx and ToolHelp parent snapshot live | Executed: run 38604699769, then 40301536939 |
| WSL1 | Linux adapter, WSL detection/semantic safeguards | Pure parser fixtures only | Not tested in WSL1 | No WSL runner; `scripts/verify-wsl.sh` is the procedure |
| WSL2 | Linux adapter, explicit WSL2 marker detection | Pure parser fixtures only | Not tested in WSL2 | No WSL runner; `scripts/verify-wsl.sh` is the procedure |

| BSD and other OS | Out of v0.1 scope | Compile-time unsupported | Not tested | None |

On WSL the Linux adapter runs, so packages and locale are expected to work,
while battery, display and desktop have nothing to answer and should report
`unsupported`. The script checks exactly that, including that `environment.kind`
says `wsl`.

"Executed" means the release binary ran on a real kernel of that platform and returned the JSON above; it does not mean every distribution, device or hardware configuration has been tried. What the runners establish is that the Linux `os-release`/`proc`/sysfs and parent-shell paths and the Windows registry/memory/ToolHelp paths work on at least one real system each. Arch, Debian, Fedora, Gentoo, openSUSE, NixOS, Intel Macs, WSL and container guests remain unexercised. ARM64 Linux has had one non-interactive run, described in its row, which is not a distribution or interactive check.

### Display (Phase 12)

| Platform | Status | Evidence |
| --- | --- | --- |
| macOS ARM64 | Supported and tested; not in the default module list | CoreGraphics: `1470x956 @60Hz`, built in, matching the logical size macOS reports. Executed on Mac15,12. The query costs ~60 ms on this host (WindowServer round trip), so it is opt-in |
| macOS Intel | Implemented, untested | Same API |
| Linux | Implemented, untested | DRM connectors and their preferred mode, **marked estimated**: sysfs does not know which mode the compositor is using. Monitor names are connector names (`card0-eDP-1`), not panel names |
| Windows | Implemented, untested | `EnumDisplayMonitors` with `GetMonitorInfoW`; the desktop rectangle is the resolution, and the refresh rate comes from `EnumDisplaySettingsW` |

`DisplayInfo` carries a `definition` saying what `width`/`height` measure,
because the platforms differ; `kind` is `built_in` or `unknown` only, since
CoreGraphics cannot tell an external panel from a virtual or remote one.

The panel's native resolution is not reported on macOS: CoreGraphics exposes the
current mode, not the panel's EDID. A scaled MacBook mode therefore shows the
logical size (1470x956), while `system_profiler` shows the panel (2560x1664).

### GPU (Phase 12)

| Platform | Status | Evidence |
| --- | --- | --- |
| macOS ARM64 | Supported and tested | IOKit `IOAccelerator`; reports `Apple M3`, vendor Apple, class `AGXAcceleratorG15G`, kind `integrated`; matches `system_profiler SPDisplaysDataType` (`Chipset Model: Apple M3`) |
| macOS Intel | Implemented, untested | Same API; Intel and AMD accelerator class names give the vendor and kind |
| Linux | Implemented, untested | DRM sysfs: PCI id, driver, and a model name only where the NVIDIA driver publishes one. No Linux machine available |
| Windows | Implemented with a documented limitation, untested | `EnumDisplayDevicesW`. Lists adapters attached to a display; an adapter with no display attached is not listed. `windows-sys` ships no DXGI bindings and a second binding crate was not added for one module |

An adapter the platform does not name is reported by vendor and id rather than
by a guessed model name: no PCI-id-to-marketing-name table is used. On macOS
the accelerator class name goes in `driver`, because that is what it is.

### Battery (Phase 12)

| Platform | Status | Evidence |
| --- | --- | --- |
| macOS ARM64 | Supported and tested | IOKit power-source descriptions; compared against `pmset -g batt` on Mac15,12 (100%, charged, drawing from AC), which matched |
| macOS Intel | Implemented, untested | Same API; no Intel hardware available |
| Linux | Implemented, untested | `/sys/class/power_supply` (`type`, `capacity`, `status`, adapter `online`); no Linux machine available |
| Windows | Implemented, untested | `GetSystemPowerStatus`; one aggregate reading, not per-pack |

A machine with no battery — a desktop, a container, most cloud VMs — reports
`unsupported` with a reason, never `0%`. A unit whose charge figure is absent
reports no percentage rather than zero.

One behaviour the Windows runner made concrete: `cargo run` is the parent there, because Cargo replaces itself with `exec` on Unix but must spawn a child on Windows. Shell detection therefore resolved to `bash` as `current_parent` on Linux and fell back to `cmd` as `configured_default` on Windows, which is the documented immediate-parent-only semantics working as intended rather than a platform defect.

### Packages (Phase 12)

| Platform | Status | Counts |
| --- | --- | --- |
| macOS ARM64 | Supported and tested | Homebrew formulae and casks as separate entries: 883 and 44, matching `brew list --formula` and `--cask` on Mac15,12 |
| Linux | Implemented, untested | Entries with `Status: install ok installed` in `/var/lib/dpkg/status`, and directories in `/var/lib/pacman/local`. RPM and Nix report unsupported with a reason |
| Windows | Implemented, untested | Directories in `scoop/apps`. winget reports unsupported: it has no local package database |

Counts are per manager and never summed; each entry carries a `definition`.

### Desktop (Phase 12)

| Platform | Status | Evidence |
| --- | --- | --- |
| Linux | Implemented, untested on a real desktop | Pure detector in `detect::desktop` with unit tests covering agreement, a single hint, a compositor socket, a conflicting session, a TTY, an unknown identifier and the display fallback |
| macOS | Unsupported by design | Aqua is the window server, not a session the user chooses |
| Windows | Unsupported by design | The shell is part of the system |

### Locale (Phase 12)

| Platform | Status | Evidence |
| --- | --- | --- |
| macOS ARM64 | Supported and tested | Process `zh_CN.UTF-8` from `LANG`, system `zh_CN` from `CFPreferences AppleLocale`, matching `defaults read -g AppleLocale` |
| Linux | Implemented, untested | Environment plus `/etc/locale.conf`, falling back to `/etc/default/locale` |
| Windows | Implemented, untested | Environment plus `GetUserDefaultLocaleName` |

The process locale and the system default are reported separately.

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
| Swap | /proc/meminfo SwapTotal and SwapFree; zero swap is reported as zero, not as missing | sysctl vm.swapusage | Unsupported: Windows reports a commit limit, not page-file usage, and the derivation from it does not hold |
| Disk | statvfs on $HOME, falling back to / when HOME is unset | statvfs on $HOME | GetDiskFreeSpaceExW on %USERPROFILE% |
| Terminal | TERM_PROGRAM and TERM_PROGRAM_VERSION, then WT_SESSION, then TERM. The first two name the application and are confirmed; TERM names a capability type, is reported as `capability_type` and marked estimated. One shared path for all three platforms | same | same |
| Local IP | getifaddrs: IPv4 on an interface that is up, excluding loopback, link-local and unspecified through one shared predicate | same | GetAdaptersAddresses: IPv4 unicast on an adapter whose OperStatus is up, through the same predicate |

Disk reports the filesystem containing the queried path. `used_bytes` is `total - free` and `available_bytes` is what the calling user may still allocate; they are reported separately rather than derived from each other, because the two coincide on a filesystem with no reserved blocks and diverge on one with them. On APFS the kernel reports the container's block counters rather than a single volume's, so `used` here is the container's total minus its free space, while `df` lists each volume separately. The two answer different questions, and they differ by a small amount that `df` attributes to no volume; `docs/validation.md` records the figures rather than smoothing them. Local IP lists every address instead of choosing one, because whichever interface the kernel reports first is often a container bridge rather than the one carrying traffic.

`--privacy` treats two kinds of value differently, and the difference is deliberate. The user name, the host name and the local addresses are **withheld**: their fields keep their slot and report `redacted`, with no value at all, so a consumer can tell a withheld value from an invented one. A Host row whose kind is `hostname` is withheld the same way, because it would otherwise repeat the name just hidden; a device-model Host stays, since it identifies hardware rather than a person. Shell and terminal are left alone.

The disk path is **shortened**, not withheld: it becomes `~` and the field stays `available` and `confirmed`, because the counters are exact and `~` is an accurate shorthand for the directory that was read. A consumer that reads only `status` therefore cannot tell the path was shortened, which is the intended trade — the path is a label, the numbers are the measurement, and marking an exact reading as redacted would misstate it. Configuration paths follow the platform: `$XDG_CONFIG_HOME/justfetch/config.toml` or `~/.config/justfetch/config.toml` on Linux and macOS, `%APPDATA%\justfetch\config.toml` on Windows. A missing file is not an error; a file that exists and does not parse is. See `docs/configuration.md`.

Windows reports no page-file figure directly. The obvious substitute — commit charge above resident memory — was implemented, measured on a runner and **rejected**: committed memory came out at 2.41 GiB against 2.62 GiB resident, so the subtraction went negative, because file-backed mappings and driver memory are resident without being committed. WMI would answer the question but is an out-of-process query, and PDH is a separate subsystem; neither is something this project will reach for one number. The field therefore reports `unsupported` with that reason, and `docs/validation.md` records the counters that settled it.

Host has the most involved fallback chain, and its exact behavior differs by platform and failure kind. Shell (parent executable, then `SHELL`/`ComSpec`), Terminal (`TERM_PROGRAM`, then `WT_SESSION`, then `TERM`), Linux OS (`/etc/os-release`, then `/usr/lib/os-release`) and Disk (`$HOME`, then `/`) also fall back. Shell, Terminal and OS label which step answered, through a `kind` or through the `source` field, so a fallback is never presented as the primary answer. Disk has no label to give: the path it reports is the one it read, so a reader sees `/` rather than being told that a query for `$HOME` succeeded. On Linux an absent `/sys/devices/virtual/dmi/id/product_name` (and absent device-tree model) falls through to the labeled hostname, as does a placeholder string such as `System Product Name`; if the file exists but cannot be read, is not UTF-8, or exceeds 64 KiB, the read error is reported for Host instead of being downgraded to a hostname. On macOS `hw.model` is the sole source and there is no hostname fallback: an unreadable, empty or non-UTF-8 value is reported as an error. On Windows Host is always the labeled hostname and no WMI or model query is attempted. The fallback never invents a model, and an error is never rewritten as a successful hostname.

Every field has available/unavailable/unsupported/redacted/deselected/error status. `deselected` means the module was not selected, so nothing was attempted; it is not a failure and not a platform limitation. Windows swap is the one field that reports `unsupported`: the platform exposes no page-file usage through any API this project permits, which is a different claim from a value that is merely unavailable right now. It renders as `Unsupported` and carries its reason. Available fields have source/confidence and optional note. Some sysinfo APIs report only Option, so unavailable can include library-level access/detection failure; justfetch cannot recover an error the dependency does not expose. Direct native calls and file reads preserve errors. A field failure does not suppress other results or make JSON invalid. Exit 0 means collection/rendering completed; inspect field status to assess completeness. Invalid CLI usage and output-write failures return nonzero; broken pipes exit quietly.

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

Colour depth follows what the terminal advertises: true colour when COLORTERM contains `truecolor` or `24bit`, the nearest xterm-256 index when TERM contains `256color`, and the nearest of the sixteen basic colours otherwise. The 16-colour step is an approximation — it maps against the xterm palette, which a terminal or theme may redefine — so a built-in theme that is published in true colour still renders legibly but not identically there. The default theme asks only for named colours and is unaffected. Layouts degrade by width: rules and swatches below 24 columns, artwork below the width it and 36 information cells need. None of this changes what is measured or what the JSON contains.

Untrusted ESC/CSI/OSC/DCS strings and control/bidi characters are removed before terminal layout. JSON escapes control characters but retains original values; downstream clients must sanitize before displaying JSON values in terminals. User/host identity can be sensitive and is visible by default. Username is an environment hint, marked estimated. A configured-default shell is labeled `~ name (default)` and is not evidence of the currently interactive shell.

## Known validation limits

WSL, Intel Macs, containers, and every distribution other than Ubuntu remain unexercised, as does any path that depends on particular hardware; the ARM64 Linux row records the one narrow exception. The matrix above records exactly what was run and where; where a row says a check ran, it ran on that one configuration and nothing wider. Cross-compilation checks prove compilation only, not linking, terminal appearance or system-call behavior. See validation.md for exact commands and results.
