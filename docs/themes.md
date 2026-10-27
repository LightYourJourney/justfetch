# Themes, layouts and logos

justfetch ships a default look that needs no configuration, four ways to arrange
the same information, and six colour themes. This page is the visual
specification: what the design tries to achieve, how a theme is expressed, and
what happens when the terminal cannot show what a theme asks for.

## Design rules

A terminal gives no control over type size, weight or spacing beyond what the
font already does. Hierarchy therefore has to come from four things, in this
order of strength:

1. **Position.** The title is first, the information follows, the palette sits
   last. A reader finds the name before the numbers without being told.
2. **Alignment.** Values share one left edge, so a column of numbers can be
   compared at a glance. The column is derived from the labels, not fixed, so a
   custom label widens it instead of breaking it.
3. **Colour.** One accent marks everything that identifies a row: the title and
   the label column. Values stay in the terminal's own foreground, which keeps
   long text readable and lets the accent mean "this is structure".
4. **Rules and whitespace.** A single rule under the title and another above the
   palette close the block. They are the first decoration dropped when width is
   short, because they carry no information.

Two rules follow from those:

- Colour is added after every width is measured. No escape sequence can shift a
  column, and removing the escapes from a themed run leaves exactly the plain
  one. The single exception is the swatch row, which is the theme's own palette
  and may have a different number of chips.
- Nothing is truncated to make the picture fit. A long value wraps with its
  label hanging indented; a narrow terminal loses decoration, then artwork, and
  keeps the information.

## Layouts

| Layout | Artwork | Title | Rules | Swatches | Label column | Intended for |
| --- | --- | --- | --- | --- | --- | --- |
| `classic` (default) | beside, when it fits | yes | yes | yes | every module's label | the normal screenshot case |
| `compact` | no | yes | no | no | the labels actually shown | dense terminals, many rows |
| `minimal` | no | yes | no | no | `Label: value`, unaligned | narrow panes, piping, diffs |
| `vertical` | above, centred | yes | yes | yes | every module's label | wide terminals, tall artwork |

`classic` and `vertical` drop automatically chosen artwork when the column would
fall below 36 cells of information beside it (`classic`) or when the artwork
leaves fewer than four spare columns (`vertical`). Artwork the user named
explicitly — `--logo NAME`, `--logo-file` or the matching configuration keys —
is not dropped while it fits the terminal: `classic` moves it above the
information instead, the same arrangement `vertical` uses. Artwork wider than
the terminal itself cannot be drawn and is omitted. To keep the `classic`
two-column arrangement without artwork, say `--logo none`; to put it above
deliberately, say `--layout vertical`. Both drop the rules and swatches below 24
columns. `compact` deliberately narrows the label column to the rows on screen,
so a three-module selection does not pay for `Kernel`'s width.

Select a layout with `--layout NAME` or `[layout] name` in the configuration
file. The setting is one value, not a pile of booleans: a layout owns its own
arrangement, and a theme never changes it.

## Themes

A theme is a table of colours. It contains no collection code, no formatting and
no layout decisions — adding one is a data change, never a second rendering
path.

| Role | What it colours | Unset means |
| --- | --- | --- |
| `title` | `user@host` | the accent (the logo colour) |
| `label` | the label column | the accent |
| `value` | the values | the terminal's foreground |
| `divider` | the two rules | the terminal's foreground |
| `emphasis` | the `~` on an estimated value | the terminal's foreground |
| `logo` | the artwork | each logo's own distributed colour |
| `logo_secondary` | the other end of the artwork shading | no shading |
| `swatches` | the palette chips under the block | — (each built-in theme defines them) |

`[color] accent` overrides `title` and `label` for any theme — it is the user
saying what those two roles are, so it wins over the theme rather than merely
filling a gap.

Where both `logo` and `logo_secondary` are set and the terminal can show it, the
artwork is shaded top to bottom between the two. Named 16-colour values are not
blended — there is no defined midpoint between `red` and `blue` — so the first
lines take the primary and the last take the secondary.

### Built-in themes

| Name | Character |
| --- | --- |
| `default` | the terminal's own sixteen colours; artwork keeps its distribution colour |
| `catppuccin-mocha` | pastels on a muted base — mauve artwork, blue labels |
| `nord` | arctic blues and greys with one warm accent |
| `tokyo-night` | deep blue base, neon accents |
| `gruvbox` | warm retro amber, clay and moss |
| `monochrome` | greys only, for terminals with one accent |

`justfetch themes` lists them with a colour preview: the theme's name is drawn
in its title colour, followed by its palette and a sample of the label, value,
rule, emphasis and artwork colours (`Aa Aa ── ~ ██`), with a one-line
description wrapped to the terminal width.
`justfetch --format json themes` returns the same list as JSON.
`--theme NAME` selects one; `[theme] name` does the same in a file. `catppuccin`
is accepted for `catppuccin-mocha`, and `tokyonight` for `tokyo-night`.

### Colour capability

A theme names the colour its design is published in, usually true colour. What
the terminal receives is decided once per run:

| Terminal | What is emitted |
| --- | --- |
| `COLORTERM` contains `truecolor` or `24bit` | the exact `#rrggbb` |
| `TERM` contains `256color` | the nearest xterm-256 index |
| anything else that accepts colour | the nearest of the sixteen basic colours |
| not a terminal, `NO_COLOR` set and nonempty, `TERM=dumb`, `--no-color` | nothing |

The 16-colour reduction is an approximation: it maps against the xterm palette,
which terminals are free to redefine. That is why the default theme uses named
colours — it asks for nothing a terminal has to guess at — and why the other
themes are still legible when reduced: roles differ in lightness as well as hue
where possible.

### Custom themes

`[theme] file` names a TOML file, relative to the configuration file that names
it; `--theme-file PATH` does the same from the command line, so a custom theme
does not require a configuration file. Every key except `name` is optional. A
complete example of both files is in [`docs/examples/`](examples/).

```toml
name = "midnight"
description = "My terminal's own palette"

title = "#c0caf5"
label = "bright-blue"
value = "default"
divider = "bright-black"
emphasis = "#e0af68"

logo = "#7aa2f7"
logo_secondary = "#bb9af7"

swatches = ["#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68"]
```

Colours are `#rrggbb`, a basic ANSI name, `bright-` plus a name, or `default`.
Unknown keys, an empty `name`, an unparseable colour and a `swatches` list
outside 1–8 entries are errors that name the file and the key. Theme files over
16 KiB are refused before parsing.

## Logos

| Selection | Meaning |
| --- | --- |
| `auto` (default) | the detected distribution, or the generic Linux emblem, or the fallback |
| `none` | no artwork |
| a built-in name | `macos`, `windows`, `linux`, `arch`, `debian`, `ubuntu`, `fedora`, `gentoo`, `opensuse`, `nixos`, `fallback`; `archlinux` and the openSUSE edition ids are accepted for their parents |
| `--logo-file PATH` / `[layout] logo_file` | custom ASCII or Unicode artwork |

Built-in artwork is licensed with this project (see `assets/logos/README.md`);
no Nerd Font or private-use glyph is required.

A custom file is user input and is treated as such:

- at most 32 KiB, 40 lines and 160 columns per line — anything larger is an
  error rather than silently truncated artwork;
- terminal escape sequences are stripped, so artwork copied from a screenshot
  still renders but cannot drive the terminal;
- trailing spaces are removed and leading ones kept, so the drawing keeps its
  alignment; CRLF is accepted;
- an empty file is an error.

The renderer colours the artwork with the theme's `logo` role, falling back to
the built-in artwork's own colour when the theme does not set one.

## What is not claimed

Colour names, boldness and the exact rendering of `●` depend on the font and the
terminal's palette. The tests pin the text, the alignment and the escape
sequences the program emits; they cannot prove how a particular terminal draws
them. `docs/validation.md` records what was actually exercised.
