use crate::model::OsFamily;
use anstyle::AnsiColor;

pub struct Logo {
    pub name: &'static str,
    pub art: &'static str,
    pub color: AnsiColor,
}

/// Artwork for every Linux distribution without a design of its own.
const LINUX: (&str, &str, AnsiColor) = (
    "linux",
    include_str!("../../assets/logos/linux.txt"),
    AnsiColor::BrightYellow,
);

/// Select artwork for a detected distribution id. `family` decides what an
/// unrecognized Linux host gets: the generic Linux logo rather than the
/// unidentified-system fallback, so a Linux user is never shown the wrong
/// emblem.
pub fn select(id: Option<&str>, family: Option<&OsFamily>) -> Logo {
    let (name, art, color) = match id.unwrap_or("") {
        "macos" => (
            "macos",
            include_str!("../../assets/logos/macos.txt"),
            AnsiColor::BrightWhite,
        ),
        "windows" => (
            "windows",
            include_str!("../../assets/logos/windows.txt"),
            AnsiColor::BrightBlue,
        ),
        "arch" | "archlinux" => (
            "arch",
            include_str!("../../assets/logos/arch.txt"),
            AnsiColor::BrightCyan,
        ),
        "debian" => (
            "debian",
            include_str!("../../assets/logos/debian.txt"),
            AnsiColor::BrightRed,
        ),
        "ubuntu" => (
            "ubuntu",
            include_str!("../../assets/logos/ubuntu.txt"),
            AnsiColor::BrightRed,
        ),
        "fedora" => (
            "fedora",
            include_str!("../../assets/logos/fedora.txt"),
            AnsiColor::BrightBlue,
        ),
        "gentoo" => (
            "gentoo",
            include_str!("../../assets/logos/gentoo.txt"),
            AnsiColor::BrightMagenta,
        ),
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => (
            "opensuse",
            include_str!("../../assets/logos/opensuse.txt"),
            AnsiColor::BrightGreen,
        ),
        "nixos" => (
            "nixos",
            include_str!("../../assets/logos/nixos.txt"),
            AnsiColor::BrightCyan,
        ),
        "linux" | "alpine" | "rhel" | "centos" | "rocky" | "linuxmint" | "pop" | "manjaro" => LINUX,
        _ if family == Some(&OsFamily::Linux) => LINUX,
        _ => (
            "fallback",
            include_str!("../../assets/logos/fallback.txt"),
            AnsiColor::BrightCyan,
        ),
    };
    Logo { name, art, color }
}
