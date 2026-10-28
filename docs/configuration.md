# Configuration

justfetch works without a configuration file. Everything below is optional, and a missing file is not an error — only a file that exists and does not parse is.

## Where the file lives

| Platform | Path |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/justfetch/config.toml`, or `~/.config/justfetch/config.toml` when `XDG_CONFIG_HOME` is unset |
| macOS | the same XDG path: `$XDG_CONFIG_HOME/justfetch/config.toml`, or `~/.config/justfetch/config.toml` |
| Windows | `%APPDATA%\justfetch\config.toml` |

macOS deliberately uses the XDG location rather than `~/Library/Application Support`, so one documented path covers every Unix-like platform and the file sits next to the other command-line tools it is used with. If `HOME` (or `APPDATA` on Windows) is absent, there is no user path; `--config` still works, and `init` says so.

## Precedence

Tightest first:

1. Command-line arguments (`--modules`, `--theme`, `--theme-file`, `--layout`, `--logo`, `--logo-file`, `--color`, `--no-color`).
2. The file named by `--config`.
3. The user configuration file at the path above.
4. Built-in defaults.

`--config` is layered over the user file rather than replacing it: keys it sets override the user value, and keys it omits keep the user value. Command-line values then override both.

Nothing is ever written to the user directory unless you run `justfetch init`.

## Example

```toml
# Modules to show, in display order.
modules = ["os", "kernel", "cpu", "memory", "disk"]

[theme]
# A built-in theme, or use `file = "path.toml"` for one of your own.
name = "catppuccin-mocha"

[layout]
# classic, compact, minimal or vertical.
name = "classic"
# "auto" follows the detected distribution; "none" hides the artwork; or name
# a built-in logo.
logo = "auto"
# Text between the label column and the value.
separator = "  "

[color]
# ANSI colour, only on a terminal that supports it.
enabled = true

[labels]
cpu = "Processor"
```

Running `justfetch init` writes this shape with every key at its default, so you can delete the lines you do not want to change. `init` never overwrites an existing file; it fails and says so.

## Reference

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `modules` | array of strings | all thirteen, in display order | Which rows to show, in the order given. The same list decides what is collected. |
| `theme.name` | theme identifier | `"default"` | A built-in theme: `default`, `catppuccin-mocha`, `nord`, `tokyo-night`, `gruvbox`, `monochrome`. |
| `theme.file` | path | none | A custom theme file, relative to the configuration file. Cannot be combined with `theme.name`; `--theme-file PATH` sets the same thing. |
| `layout.name` | `"classic"`, `"compact"`, `"minimal"` or `"vertical"` | `"classic"` | Where the pieces go. See [themes and layouts](themes.md). |
| `layout.logo` | `"auto"`, `"none"` or a built-in logo name | `"auto"` | `auto` follows the detected distribution and hides the artwork when the terminal is too narrow; `none` never draws it; a name pins one logo. |
| `layout.logo_file` | path | none | Custom ASCII or Unicode artwork, relative to the configuration file. Cannot be combined with `layout.logo` in the same file. |
| `layout.separator` | string, non-empty | `"  "` | Text between the label column and the value. `minimal` uses `": "` instead. |
| `color.enabled` | boolean | `true` | ANSI colour, still subject to `NO_COLOR`, `TERM=dumb`, a non-terminal stdout and the platform's ANSI support. |
| `color.accent` | string | the theme's | Overrides the title and label colours: `#rrggbb`, `black`/`red`/…/`white`, any of those prefixed `bright-`, or `default`. Omitted, the theme's own colours apply, falling back to the artwork colour. |
| `labels.<module>` | string, non-blank | the built-in label | Replaces the label shown for that module. |

Unknown keys are an error rather than being ignored, so a misspelt key is reported instead of silently doing nothing.

### Module identifiers

`os`, `host`, `kernel`, `uptime`, `cpu`, `gpu`, `memory`, `swap`, `disk`, `battery`, `shell`, `terminal`, `local_ip`.

Identifiers are case-insensitive and `local-ip` is accepted as an alias for `local_ip`. `justfetch --help` lists them too.

### What is collected

The module list decides collection, not just drawing. A module that is not listed is never queried: choosing `--modules cpu` skips the address enumeration, the filesystem query, the process lookup and the registry reads that the other rows would need. The saving is real but small — about 2% of the process's instructions for a one-module selection — because process startup dominates; the measurement and its raw data are in `docs/validation.md`.

The logo is chosen from the OS module. If `os` is not selected there is no distribution to choose from, so the generic fallback emblem is used; selecting `os` brings the real one back.

The user and host name are collected even when no module asks for them, because the title line shows both. They are cheap, and `--privacy` withholds them.

## Theme and logo files

A copyable full configuration is in [examples/config.toml](examples/config.toml),
with a custom theme beside it. `[theme] file` and `[layout] logo_file` are read relative to the configuration
file that names them, so a theme kept beside the configuration travels with it.
An absolute path is used as given. On Windows, a path inside a TOML basic
string needs its backslashes doubled (`"C:\\Users\\you\\theme.toml"`), or the
string can be written in single quotes (`'C:\Users\you\theme.toml'`); forward
slashes work too. Each file is bounded — 16 KiB for a theme, 32 KiB
for a logo — and checked by the same code that checks a `--config` file, so
`doctor` reports a broken one with the file it came from.

The theme format, the role names and the artwork rules are in
[themes and layouts](themes.md).

## Diagnostics

`justfetch doctor` reports what it can actually check: the resolved user path and whether it exists, the path given to `--config`, the effective module list and layout, and any configuration error, naming the file — and the line and column when the TOML parser supplied them, which is parse and type errors; a value that parses but is wrong, such as an unknown module, names the file and the key instead. It exits non-zero when it finds a problem and prints `no problems found` when it does not.

```
$ justfetch --config broken.toml doctor
user config: /home/you/.config/justfetch/config.toml (not present; built-in defaults apply)
--config: broken.toml (found)
problem: configuration error: broken.toml: modules: `gpu` is not a module id; expected one of os, host, kernel, uptime, cpu, gpu, memory, swap, disk, battery, shell, terminal, local_ip
```

Errors from a normal run go to stderr with a non-zero exit code, and nothing is written to stdout:

| Situation | Message |
| --- | --- |
| Unknown module | `` `gpu` is not a module id; expected one of … `` |
| Duplicate module | `` `os` is listed twice `` |
| Empty list | `the module list is empty; name at least one module or omit the setting` |
| `--config` missing | `--config PATH: file not found` |
| Unknown theme | `` expected one of default, catppuccin-mocha, … `` |
| Unknown layout | `` expected one of classic, compact, minimal, vertical `` |
| Unknown logo | `` expected `auto`, `none`, or one of macos, windows, … `` |
| `--logo` with `--logo-file` | `--logo and --logo-file ask for different things` |
| Theme file too large | `file is N bytes; the limit is 16384` |
| Logo file too large | `file is N bytes; the limit is 32768` |
| Bad theme colour | `` title is `chartreuse`; expected `#rrggbb`, a basic colour name, … `` |
| Malformed TOML | `TOML parse error at line N, column M` |
| Wrong value type | `TOML parse error …` naming the field |
| Unknown key | `unknown field` naming the key |
| Bad `layout.logo` | `` expected `auto` or `none` `` |
| Blank label | `labels.cpu is empty; omit it to keep the default label` |

## JSON

Module selection does not change the document's shape. A module that was not selected reports `"status": "deselected"` and carries no `value`, which is distinct from a module that was attempted and failed (`error`), could not be answered (`unavailable`), or is not available on the platform (`unsupported`). `schema_version` stays `1`; the value only appears when a run selects a subset.

Theme, layout, labels, separator, logo and colour are presentation only and never reach the JSON. The configuration is still validated in full for a JSON run, so a missing theme or logo file is an error there too — the file is checked, not applied.

Control characters in a label or separator are stripped before rendering, and `doctor` reports them so the file is not silently altered from what it says.
