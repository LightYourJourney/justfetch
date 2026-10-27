//! Built-in artwork and validated custom artwork.
//!
//! A logo is text plus an optional colour of its own. It never carries escape
//! sequences: custom files are sanitised on the way in, so the renderer is the
//! only code that emits styling.

use super::{
    colour::Colour,
    layout::{sanitize, width},
};
use crate::model::OsFamily;
use anstyle::AnsiColor;

/// A custom logo is user input, so it is bounded before it can affect layout.
pub const MAX_BYTES: usize = 32 * 1024;
pub const MAX_LINES: usize = 40;
pub const MAX_WIDTH: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logo {
    pub name: String,
    pub art: String,
    /// The artwork's own colour. `None` means the theme decides; a custom
    /// file has no colour of its own.
    pub color: Option<Colour>,
}

impl Logo {
    pub fn lines(&self) -> Vec<&str> {
        self.art.lines().collect()
    }
}

/// The built-in artwork, in the order `--help` and errors list it. One table:
/// detection, explicit selection and the help text all read it, so adding a
/// distribution is one row.
const ART: [(&str, &str, AnsiColor); 11] = [
    (
        "macos",
        include_str!("../../assets/logos/macos.txt"),
        AnsiColor::BrightWhite,
    ),
    (
        "windows",
        include_str!("../../assets/logos/windows.txt"),
        AnsiColor::BrightBlue,
    ),
    (
        "linux",
        include_str!("../../assets/logos/linux.txt"),
        AnsiColor::BrightYellow,
    ),
    (
        "arch",
        include_str!("../../assets/logos/arch.txt"),
        AnsiColor::BrightCyan,
    ),
    (
        "debian",
        include_str!("../../assets/logos/debian.txt"),
        AnsiColor::BrightRed,
    ),
    (
        "ubuntu",
        include_str!("../../assets/logos/ubuntu.txt"),
        AnsiColor::BrightRed,
    ),
    (
        "fedora",
        include_str!("../../assets/logos/fedora.txt"),
        AnsiColor::BrightBlue,
    ),
    (
        "gentoo",
        include_str!("../../assets/logos/gentoo.txt"),
        AnsiColor::BrightMagenta,
    ),
    (
        "opensuse",
        include_str!("../../assets/logos/opensuse.txt"),
        AnsiColor::BrightGreen,
    ),
    (
        "nixos",
        include_str!("../../assets/logos/nixos.txt"),
        AnsiColor::BrightCyan,
    ),
    (
        "fallback",
        include_str!("../../assets/logos/fallback.txt"),
        AnsiColor::BrightCyan,
    ),
];

/// Distribution ids without artwork of their own: they are Linux, so they get
/// the Linux emblem rather than the unidentified-system one.
const LINUX_FAMILY: [&str; 8] = [
    "alpine",
    "rhel",
    "centos",
    "rocky",
    "linuxmint",
    "pop",
    "manjaro",
    "archlinux-derived",
];

/// Built-in identifiers, in the order `--help` and errors list them.
pub fn built_in_names() -> Vec<&'static str> {
    ART.iter().map(|(name, _, _)| *name).collect()
}

fn artwork(name: &str) -> Option<(&'static str, AnsiColor)> {
    ART.iter()
        .find(|(id, _, _)| *id == name)
        .map(|(_, art, colour)| (*art, *colour))
}

/// The artwork for one canonical id. Only an empty table could make this fail,
/// and then the caller gets a readable fallback rather than a panic.
fn of(name: &str) -> Logo {
    let (art, colour) = artwork(name).unwrap_or((
        include_str!("../../assets/logos/fallback.txt"),
        AnsiColor::BrightCyan,
    ));
    Logo {
        name: name.to_owned(),
        art: art.to_owned(),
        color: Some(Colour::Ansi(colour)),
    }
}

/// A name a user may type: a built-in id, or the aliases the help does not list.
fn user_name(name: &str) -> Option<&'static str> {
    let canonical = match name {
        "archlinux" => "arch",
        "opensuse-leap" | "opensuse-tumbleweed" => "opensuse",
        other => other,
    };
    ART.iter()
        .find(|(id, _, _)| *id == canonical)
        .map(|(id, _, _)| *id)
}

/// A distribution id reported by the platform.
fn detected_name(id: &str) -> Option<&'static str> {
    if let Some(name) = user_name(id) {
        return Some(name);
    }
    LINUX_FAMILY.contains(&id).then_some("linux")
}

/// Select artwork for a detected distribution id. `family` decides what an
/// unrecognized Linux host gets: the generic Linux logo rather than the
/// unidentified-system fallback, so a Linux user is never shown the wrong
/// emblem.
pub fn select(id: Option<&str>, family: Option<&OsFamily>) -> Logo {
    let canonical = id.and_then(detected_name).unwrap_or_else(|| {
        if family == Some(&OsFamily::Linux) {
            "linux"
        } else {
            "fallback"
        }
    });
    of(canonical)
}

/// Look up artwork by the name a user typed. Distribution aliases are accepted
/// so `--logo archlinux` and `--logo opensuse-leap` do what they look like.
pub fn built_in(name: &str) -> Option<Logo> {
    let name = name.trim().to_ascii_lowercase();
    user_name(&name).map(of)
}

/// Validate a custom logo file's text. Pure: the caller read the file.
///
/// Escape sequences are stripped rather than rejected, so a screenshot pasted
/// into a file still renders; what cannot be stripped — an oversized file, too
/// many lines, a line wider than any terminal — is an error rather than
/// silently truncated artwork.
pub fn parse_custom(text: &str, name: &str, origin: &str) -> Result<Logo, String> {
    if text.len() > MAX_BYTES {
        return Err(format!(
            "{origin}: logo is {} bytes; the limit is {MAX_BYTES}",
            text.len()
        ));
    }
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| sanitize(line).trim_end().to_owned())
        .collect();
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        return Err(format!("{origin}: logo is empty"));
    }
    if lines.len() > MAX_LINES {
        return Err(format!(
            "{origin}: logo has {} lines; the limit is {MAX_LINES}",
            lines.len()
        ));
    }
    for (index, line) in lines.iter().enumerate() {
        let cells = width(line);
        if cells > MAX_WIDTH {
            return Err(format!(
                "{origin}: logo line {} is {cells} columns; the limit is {MAX_WIDTH}",
                index + 1
            ));
        }
    }
    Ok(Logo {
        name: name.to_owned(),
        art: format!("{}\n", lines.join("\n")),
        color: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_name_resolves_to_artwork() {
        for name in built_in_names() {
            let logo = built_in(name).unwrap_or_else(|| panic!("{name} is missing"));
            assert!(!logo.art.trim().is_empty(), "{name} is blank");
            assert_eq!(logo.name, name);
        }
        assert!(built_in("archlinux").is_some());
        assert!(built_in("Ubuntu").is_some());
        assert!(built_in("not-a-distribution").is_none());
    }

    #[test]
    fn detection_falls_back_by_family_not_by_guess() {
        assert_eq!(select(Some("ubuntu"), None).name, "ubuntu");
        assert_eq!(
            select(Some("unknown-id"), Some(&OsFamily::Linux)).name,
            "linux"
        );
        assert_eq!(
            select(Some("unknown-id"), Some(&OsFamily::Macos)).name,
            "fallback"
        );
        assert_eq!(select(None, None).name, "fallback");
    }

    #[test]
    fn custom_artwork_is_cleaned_rather_than_trusted() {
        let logo = parse_custom("  /\\\x1b[31m\n /  \\  \n\n", "mine", "logo.txt").unwrap();
        assert_eq!(logo.art, "  /\\\n /  \\\n");
        assert_eq!(logo.color, None);

        for (text, expected) in [
            ("", "empty"),
            ("\n \n", "empty"),
            ("x".repeat(MAX_BYTES + 1).as_str(), "the limit is"),
            ("line\n".repeat(MAX_LINES + 1).as_str(), "lines"),
            ("x".repeat(MAX_WIDTH + 1).as_str(), "columns"),
        ] {
            let error = parse_custom(text, "mine", "logo.txt").unwrap_err();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn only_trailing_space_is_removed() {
        let logo = parse_custom("  a  \n b\t\n", "mine", "logo.txt").unwrap();
        assert_eq!(logo.art, "  a\n b\n");
    }
}
