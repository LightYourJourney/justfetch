# Security policy

## Reporting a vulnerability

Open a private security advisory on the repository
(<https://github.com/LightYourJourney/justfetch/security/advisories/new>) rather
than a public issue. Include the version (`justfetch --version`), the platform,
and the smallest reproduction you have. This is a small project without a
staffed security team: expect a best-effort response, and please do not publish
details before a fix is available.

## Supported versions

| Version | Supported |
| --- | --- |
| 1.0.x (including release candidates) | Yes |
| 0.1.x | No; it was a pre-release with seven fields |

## What the program does, and what it does not

The design decides most of the threat model:

- **No network.** justfetch makes no runtime requests. The only outbound
  connections in this repository are the CI workflows' own operations, which
  run in Actions, not in the binary.
- **No subprocesses.** Nothing is executed; the shell is identified by the
  parent process image, not by running `ps` or a shell.
- **No privilege.** No administrator or root access is requested or needed.
- **Bounded reads.** The binary reads system paths the platform documents
  (`/proc`, `/sys`, `/etc/os-release`, `/var/lib/dpkg/status`,
  `/var/lib/pacman/local`, `/Library`, the IORegistry) and the user's own
  configuration, theme and logo files. It does not walk directories generally.
- **Writes only on request.** `justfetch init` writes one file, and refuses to
  overwrite an existing one.

## Inputs treated as untrusted

Every string that reaches the terminal is sanitised before it is measured or
styled: system values, environment variables, custom labels, separators, theme
names and colours, and custom logo files. Escape sequences, control characters
and bidirectional overrides are removed, so a hostile hostname or a crafted
config value cannot drive the terminal, and diagnostics are sanitised too, so a
configuration value cannot forge a line of output.

Theme, logo and configuration files are parsed as data. They cannot execute
code. Sizes are bounded (16 KiB for a theme, 32 KiB for a logo) and malformed
input is reported with the file and the key rather than ignored.

## Relevant findings

- RUSTSEC-2022-0008 and RUSTSEC-2024-0402 affect transitive crates
  (`windows`, `hashbrown`); the locked versions are above the patched versions.
  The advisory database was checked against `Cargo.lock` when this file was
  written.
- An earlier round fixed a macOS crash: `AppleLocale` is user-writable, and a
  number there made a Core Foundation string call throw, aborting the process.
  Every Core Foundation read now checks the value's type first.

## Out of scope

- Information a user chooses to display: `--privacy` withholds identity and
  addresses, but system details themselves are the program's purpose.
- Terminals that misrender text, fonts without the required glyphs, and
  colour palettes the user's terminal defines.
- Builds produced from modified sources, and binaries not downloaded from this
  repository's releases.
