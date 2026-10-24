# Architecture — v0.1

justfetch is a single Rust crate with a small library for deterministic tests and a thin CLI binary. Scope: eleven system fields, terminal and JSON output, automatic original ASCII logos, and no runtime network/configuration/subprocesses.

## Data flow

`CLI → Config → detect::collect → platform adapter → SystemSnapshot → renderer → stdout`

The collector owns all system access. Renderers receive an immutable snapshot and explicit terminal capabilities; they never inspect the environment or call system APIs. CLI derives only effective output options. v0.2 will extend configuration here without changing detection.

## Model

Each `Detection<T>` is tagged `available`, `unavailable`, `unsupported`, or `error`. Available results carry `source`, `confidence` (`confirmed` or `estimated`), and optional explanatory notes. Errors carry a stable code and source; the enclosing field identifies the module. Missing is distinct from an API failure. JSON has `schema_version: 1`, original seconds/bytes, and structured host/shell/memory semantics. Additive fields may appear; consumers should ignore unknown fields. Breaking schema changes increment the schema version.

Host is a device model or a hostname with explicit `kind`; identity is separately collected as username and hostname. Environment identity is best effort and marked estimated. Shell is a recognized immediate parent executable, or the explicitly labeled configured default from SHELL/ComSpec. It is never claimed to identify arbitrary ancestor shells. Linux distro and WSL environment are separate from Windows host hardware.

## Detection

Use sysinfo with only its `system` feature, selective CPU-brand refreshes, no usage sampling and no rayon. Memory uses checked native calls/parsers rather than infallible refresh methods. Audit the pinned implementation for units and memory semantics. Native Linux file parsers retain errors and missing states; macOS sysctl obtains exact hardware model; Windows uses sysinfo native APIs and hostname fallback rather than WMI. Small audited FFI wrappers are allowed only for native calls unavailable through safe interfaces. Parent-shell lookup uses getppid + /proc/exe on Linux, getppid + proc_pidpath on macOS, and one ToolHelp snapshot to find our parent ID then a single QueryFullProcessImageNameW on Windows. The Windows snapshot includes process IDs globally, but stops iteration at the current process and does not collect metadata for other processes. This is the documented-API tradeoff; sysinfo would enumerate the full table twice on macOS/Windows, so it is not used for process collection. Windows logical processor count uses GetActiveProcessorCount across all processor groups. No external commands in the runtime path.

Linux memory: total minus MemAvailable; old-kernel fallback explicitly estimated. macOS and Windows definitions are documented in compatibility.md. WSL memory/uptime describe the exposed environment, never physical Windows host claims. Only an explicit WSL2 kernel marker identifies version 2; generic Microsoft markers identify WSL with unknown version.

## Rendering

Original built-in ASCII assets; no copied third-party logos. Colors are basic ANSI, only when stdout is a suitable terminal, NO_COLOR is absent/empty, TERM is not dumb, and no-color is false. Plain text uses no escape sequences. Width is measured before styling with unicode-width; wrapping operates on grapheme boundaries. OS strings are stripped of terminal controls and bidi overrides before terminal output; JSON preserves raw strings through JSON escaping. Narrow terminals drop the logo. Tall/short logo layouts use the maximum row count. Output writes handle broken pipes cleanly.

## Implementation plan

1. CLI contract tests and typed snapshot with status serialization.
2. Platform collection, pure Linux parsers, precise host/shell/memory semantics.
3. Original logos, capability policy, safe wrapping and styled rendering fixtures.
4. Full CLI integration, three-OS CI, documentation and release build.
5. Measured hyperfine results, self-review, independent review and regression fixes.

## Validation

Parser and state tests use deterministic inputs. Render tests use a fixed snapshot, including wide/combining characters and hostile controls. CLI tests execute the actual binary and validate JSON/stdout/stderr/exit codes. Local checks: fmt, clippy with warnings denied, tests, release build. CI repeats on Linux/macOS/Windows. Report actual hardware execution separately from compilation and CI configuration. No commits, remote pushes or publication are needed for this task.

## Decisions and alternatives

Native-only collection would minimize dependencies but duplicate platform maintenance and require more FFI. A full libmacchina frontend would delegate too much semantic control. Selective sysinfo plus narrow adapters balances correctness and maintenance. A single library + binary package makes fixtures easy without creating a multi-crate workspace. TOML, themes, doctor and extra detection fields remain roadmap items.

Existing GPLv3 LICENSE is preserved; package license is GPL-3.0-only. Upstream reference code/assets are not copied. See research.md for evidence and licenses.
