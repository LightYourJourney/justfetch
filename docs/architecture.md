# Architecture — v0.1

justfetch is a single Rust crate with a small library for deterministic tests and a thin CLI binary. Scope: eleven system fields, terminal and JSON output, automatic original ASCII logos, a TOML configuration file and module selection, and no runtime network or subprocesses. The only files read outside system paths are the configuration file — at the documented location or the path given to `--config` — and the theme or logo files it names, or that `--theme-file` and `--logo-file` name. Nothing is written unless `init` runs.

## Data flow

`CLI + TOML → Config (precedence) → detect::collect(&modules) → platform adapter → SystemSnapshot → privacy::redact (only with --privacy) → renderer → stdout`

The collector owns all system-information access. The one exception is the terminal capability query (`terminal_columns`, `supports_ansi`), which `Config` calls because it decides how to render, not what to collect; it reads no data into the snapshot. Renderers receive an immutable snapshot and explicit terminal capabilities; they never inspect the environment or call system APIs. Privacy is a separate pass over the finished snapshot, so the text and JSON renderers cannot disagree about what was withheld. CLI derives only effective output options. v0.2 will extend configuration here without changing detection.

## Model

Each `Detection<T>` is tagged `available`, `unavailable`, `unsupported`, `redacted`, `deselected`, or `error`. `redacted` only appears when `--privacy` was passed, and means the value was collected and is deliberately withheld — not that it was missing. `deselected` means the module was not in the selection, so nothing was attempted and nothing failed. Available results carry `source`, `confidence` (`confirmed` or `estimated`), and optional explanatory notes. Errors carry a stable code and source; the enclosing field identifies the module. Missing is distinct from an API failure. JSON has `schema_version: 1`, original seconds/bytes, and structured host/shell/memory semantics. Additive fields may appear; consumers should ignore unknown fields. Breaking schema changes increment the schema version. Two status values have been added since v1: `redacted`, which only appears when the caller passes `--privacy`, and `deselected`, which only appears when the caller selects a subset of modules. Both are opt-in, so a consumer that uses neither flag sees exactly the v1 set; that is why the version stays at 1 rather than being raised.

Host is a device model or a hostname with explicit `kind`; identity is separately collected as username and hostname. Environment identity is best effort and marked estimated. Shell is a recognized immediate parent executable, or the explicitly labeled configured default from SHELL/ComSpec. It is never claimed to identify arbitrary ancestor shells. Linux distro and WSL environment are separate from Windows host hardware.

## Detection

Use sysinfo with only its `system` feature, selective CPU-brand refreshes, no usage sampling and no rayon. Memory uses checked native calls/parsers rather than infallible refresh methods. Audit the pinned implementation for units and memory semantics. Native Linux file parsers retain errors and missing states; macOS sysctl obtains exact hardware model; Windows uses sysinfo native APIs and hostname fallback rather than WMI. Small audited FFI wrappers are allowed only for native calls unavailable through safe interfaces. Parent-shell lookup uses getppid + /proc/exe on Linux, getppid + proc_pidpath on macOS, and one ToolHelp snapshot to find our parent ID then a single QueryFullProcessImageNameW on Windows. The Windows snapshot includes process IDs globally, but stops iteration at the current process and does not collect metadata for other processes. This is the documented-API tradeoff; sysinfo would enumerate the full table twice on macOS/Windows, so it is not used for process collection. Windows logical processor count uses GetActiveProcessorCount across all processor groups. No external commands in the runtime path.

Linux memory: total minus MemAvailable; old-kernel fallback explicitly estimated. macOS and Windows definitions are documented in compatibility.md. WSL memory/uptime describe the exposed environment, never physical Windows host claims. Only an explicit WSL2 kernel marker — `wsl2` or `microsoft-standard` in the release, both of which WSL2 appends and WSL1 does not — identifies version 2; generic Microsoft markers, or the `WSL_INTEROP`/`WSL_DISTRO_NAME` environment hints alone, identify WSL with unknown version and never a guessed WSL1.

## Rendering

Original built-in ASCII assets; no copied third-party logos. Colors are basic ANSI, only when stdout is a suitable terminal, NO_COLOR is absent/empty, TERM is not dumb, and no-color is false. Plain text uses no escape sequences. Width is measured before styling with unicode-width; wrapping operates on grapheme boundaries. OS strings are stripped of terminal controls and bidi overrides before terminal output; JSON preserves raw strings through JSON escaping. Narrow terminals drop the logo. Tall/short logo layouts use the maximum row count. Output writes handle broken pipes cleanly. Field rows are produced by walking the configured module list, so selection, ordering and relabelling are decided by data rather than by a hardcoded sequence: `ModuleId` names a module once, `field_rows` maps the selection to rows, and `value_for` has one arm per module so a new module is a compile error there rather than a row that silently renders nothing. The label column width is derived from the labels themselves, so a custom label widens the column instead of breaking alignment.

## Modules and configuration

`src/module.rs` holds the identifier enum. Selection, the TOML file, labels, diagnostics, collection and rendering all key off it, and the compiler demands the arms a new variant needs. Two things are still lists rather than variants: the identifier and default label strings inside `ModuleId`, and the module list in `--help`. The first is the canonical definition; the second is checked against `ModuleId::ALL` by a test, so it cannot drift silently.

Collection is selection-driven end to end. `detect::collect` receives the ordered module list and does not call a collector that is not wanted, and each platform adapter follows the same rule down to the individual system call: an unselected module is never queried, so this is not a rendering filter. The user and host name stay unconditional because the title line shows them.

`src/config.rs` owns the file format, the platform paths, the error messages and the precedence chain (command line, `--config`, user file, defaults). Precedence is applied in one function, `resolve_with`, so the CLI path and the tests exercise the same code. `docs/configuration.md` is the user-facing reference.

`src/commands.rs` implements the subcommands — `init`, `doctor` and `themes` — with the text they print. They sit outside `config.rs` because they change for a different reason: the configuration module follows the file format and the precedence rules, while these follow what the commands say. `doctor` calls the same `resolve` a normal run does, so it reports what would actually happen rather than a second opinion.

## Themes and layouts

`src/render/theme.rs` is a table of colours per role and nothing else: it never
collects, formats or arranges. `src/render/colour.rs` decides what the terminal
is actually told — the exact RGB, the nearest xterm-256 index, the nearest of
sixteen, or nothing — from a depth the caller detected, so the conversion is a
pure function. `src/render/layout.rs` owns the four arrangements. The renderer
composes a line out of styled spans; measuring happens on plain text before any
style is attached, which is why a themed run is byte-identical to a plain one
after the escapes are removed. Adding a theme is data; adding a layout is one
arm. Neither reads the environment or touches the filesystem: configuration
reads a theme or logo file and hands the text over already bounded.

## Implementation plan

1. CLI contract tests and typed snapshot with status serialization.
2. Platform collection, pure Linux parsers, precise host/shell/memory semantics.
3. Original logos, capability policy, safe wrapping and styled rendering fixtures.
4. Full CLI integration, three-OS CI, documentation and release build.
5. Measured hyperfine results, self-review, independent review and regression fixes.

## Validation

Parser and state tests use deterministic inputs. Render tests use a fixed snapshot, including wide/combining characters and hostile controls. CLI tests execute the actual binary and validate JSON/stdout/stderr/exit codes. Local checks: fmt, clippy with warnings denied, tests, release build. CI repeats on Linux/macOS/Windows. Report actual hardware execution separately from compilation and CI configuration. No commits, remote pushes or publication are needed for this task.

## Decisions and alternatives

Native-only collection would minimize dependencies but duplicate platform maintenance and require more FFI. A full libmacchina frontend would delegate too much semantic control. Selective sysinfo plus narrow adapters balances correctness and maintenance. A single library + binary package makes fixtures easy without creating a multi-crate workspace. TOML configuration, module selection, colour themes, four layouts, custom logo files and the `init`/`doctor`/`themes` subcommands are implemented; shareable sample configurations and extra detection fields remain roadmap items.

Existing GPLv3 LICENSE is preserved; package license is GPL-3.0-only. Upstream reference code/assets are not copied. See research.md for evidence and licenses.
