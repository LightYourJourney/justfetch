use anstyle::AnsiColor;

pub struct Logo {
    pub name: &'static str,
    pub art: &'static str,
    pub color: AnsiColor,
}

pub fn select(id: Option<&str>) -> Logo {
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
        "linux" | "alpine" | "rhel" | "centos" | "rocky" | "linuxmint" | "pop" | "manjaro" => (
            "linux",
            include_str!("../../assets/logos/linux.txt"),
            AnsiColor::BrightYellow,
        ),
        _ => (
            "fallback",
            include_str!("../../assets/logos/fallback.txt"),
            AnsiColor::BrightCyan,
        ),
    };
    Logo { name, art, color }
}
