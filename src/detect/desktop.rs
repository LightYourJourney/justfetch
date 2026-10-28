//! Pure desktop-session detection: evidence in, description out.
//!
//! The platform adapter gathers the environment; this decides what it means.
//! No single variable is proof — a launcher may export one, a nested session
//! exports contradictory ones, and a compositor's socket is stronger than
//! either — so every source is compared and the result says what agreed.

use crate::model::*;

/// What the environment said. Absent values are `None` rather than empty
/// strings, so "unset" and "set to nothing" cannot be confused.
#[derive(Debug, Default, Clone)]
pub struct Evidence<'a> {
    pub xdg_current: Option<&'a str>,
    pub xdg_session: Option<&'a str>,
    pub desktop_session: Option<&'a str>,
    pub sway_socket: bool,
    pub hyprland_signature: bool,
    pub session_type: Option<&'a str>,
    pub wayland_display: bool,
    pub display: bool,
}

pub fn describe(evidence: &Evidence<'_>) -> Detection<DesktopInfo> {
    let session = match evidence.session_type {
        Some("wayland") => SessionType::Wayland,
        Some("x11") => SessionType::X11,
        Some("tty") | Some("console") => SessionType::Tty,
        _ if evidence.wayland_display => SessionType::Wayland,
        _ if evidence.display => SessionType::X11,
        _ => SessionType::Unknown,
    };
    // A compositor's socket exists only while that compositor runs.
    if evidence.sway_socket {
        return Detection::available(
            DesktopInfo {
                name: "Sway".to_owned(),
                kind: DesktopKind::Compositor,
                session,
            },
            "SWAYSOCK",
        );
    }
    if evidence.hyprland_signature {
        return Detection::estimated(
            DesktopInfo {
                name: "Hyprland".to_owned(),
                kind: DesktopKind::Compositor,
                session,
            },
            "HYPRLAND_INSTANCE_SIGNATURE",
            "the signature names the compositor, not the desktop around it",
        );
    }
    // XDG_CURRENT_DESKTOP may be a colon-separated list; the last entry is the
    // most specific ("ubuntu:GNOME").
    let current = evidence
        .xdg_current
        .and_then(|value| value.rsplit(':').next());
    let mut sources: Vec<(&str, &str)> = Vec::new();
    for (key, value) in [
        ("XDG_CURRENT_DESKTOP", current),
        ("XDG_SESSION_DESKTOP", evidence.xdg_session),
        ("DESKTOP_SESSION", evidence.desktop_session),
    ] {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            sources.push((key, value));
        }
    }
    if sources.is_empty() {
        return match session {
            SessionType::Tty => Detection::unsupported("No graphical session is running"),
            _ => Detection::unavailable(
                "No desktop variable is set; XDG_CURRENT_DESKTOP, XDG_SESSION_DESKTOP and DESKTOP_SESSION are all absent",
            ),
        };
    }
    let (name, kind) = normalise(sources[0].1);
    let agreed = sources
        .iter()
        .filter(|(_, value)| normalise(value).0 == name)
        .count();
    let note = format!(
        "sources: {}",
        sources
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let info = DesktopInfo {
        name,
        kind,
        session,
    };
    if agreed >= 2 {
        Detection::available(info, "desktop session variables")
    } else {
        // One source is a hint, not a confirmation.
        Detection::estimated(info, sources[0].0, note)
    }
}

/// Map a desktop's identifiers to one display name. An identifier this build
/// does not know is kept verbatim rather than forced into a known name.
fn normalise(raw: &str) -> (String, DesktopKind) {
    let lower = raw.trim().to_ascii_lowercase();
    let (name, kind) = match lower.as_str() {
        "gnome" | "ubuntu" | "gnome-classic" | "gnome-xorg" => ("GNOME", DesktopKind::Desktop),
        "kde" | "plasma" => ("KDE Plasma", DesktopKind::Desktop),
        "xfce" | "xfce4" => ("XFCE", DesktopKind::Desktop),
        "x-cinnamon" | "cinnamon" => ("Cinnamon", DesktopKind::Desktop),
        "mate" => ("MATE", DesktopKind::Desktop),
        "lxqt" => ("LXQt", DesktopKind::Desktop),
        "budgie" => ("Budgie", DesktopKind::Desktop),
        "pantheon" => ("Pantheon", DesktopKind::Desktop),
        "deepin" => ("Deepin", DesktopKind::Desktop),
        "sway" => ("Sway", DesktopKind::Compositor),
        "hyprland" => ("Hyprland", DesktopKind::Compositor),
        "i3" => ("i3", DesktopKind::Compositor),
        _ => return (raw.trim().to_owned(), DesktopKind::Unknown),
    };
    (name.to_owned(), kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence<'a>() -> Evidence<'a> {
        Evidence::default()
    }

    #[test]
    fn two_agreeing_variables_confirm_the_desktop() {
        let detection = describe(&Evidence {
            xdg_current: Some("ubuntu:GNOME"),
            xdg_session: Some("gnome"),
            session_type: Some("wayland"),
            ..evidence()
        });
        let value = detection.value().expect("available");
        assert_eq!(value.name, "GNOME");
        assert_eq!(value.kind, DesktopKind::Desktop);
        assert_eq!(value.session, SessionType::Wayland);
    }

    #[test]
    fn one_variable_is_a_hint_not_a_confirmation() {
        let detection = describe(&Evidence {
            xdg_current: Some("KDE"),
            ..evidence()
        });
        let Detection::Available { confidence, .. } = &detection else {
            panic!("expected a value");
        };
        assert_eq!(*confidence, Confidence::Estimated);
    }

    #[test]
    fn a_compositor_socket_beats_the_variables() {
        let detection = describe(&Evidence {
            xdg_current: Some("GNOME"),
            sway_socket: true,
            session_type: Some("wayland"),
            ..evidence()
        });
        assert_eq!(detection.value().expect("available").name, "Sway");
    }

    #[test]
    fn a_nested_session_keeps_the_first_source_and_says_so() {
        let detection = describe(&Evidence {
            xdg_current: Some("sway"),
            xdg_session: Some("gnome"),
            ..evidence()
        });
        let Detection::Available {
            value,
            confidence,
            note,
            ..
        } = &detection
        else {
            panic!("expected a value");
        };
        assert_eq!(value.name, "Sway");
        assert_eq!(*confidence, Confidence::Estimated);
        let note = note.as_deref().unwrap_or_default();
        assert!(note.contains("XDG_CURRENT_DESKTOP=sway"), "{note}");
        assert!(note.contains("XDG_SESSION_DESKTOP=gnome"), "{note}");
    }

    #[test]
    fn no_session_variables_outside_a_gui_is_unsupported_not_guessed() {
        let detection = describe(&Evidence {
            session_type: Some("tty"),
            ..evidence()
        });
        assert!(matches!(detection, Detection::Unsupported { .. }));
    }

    #[test]
    fn an_unknown_identifier_is_kept_verbatim() {
        let detection = describe(&Evidence {
            xdg_current: Some("SomeNewShell"),
            ..evidence()
        });
        let value = detection.value().expect("available");
        assert_eq!(value.name, "SomeNewShell");
        assert_eq!(value.kind, DesktopKind::Unknown);
    }

    #[test]
    fn session_type_falls_back_to_the_display_variable() {
        let detection = describe(&Evidence {
            xdg_current: Some("i3"),
            display: true,
            ..evidence()
        });
        assert_eq!(
            detection.value().expect("available").session,
            SessionType::X11
        );
    }
}
