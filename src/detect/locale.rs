//! POSIX locale precedence, as a pure function.
//!
//! `LC_ALL` overrides everything, then `LC_MESSAGES`, then `LANG`. An empty
//! value is unset, which is why the filter is here rather than at each caller.

pub fn process_locale(
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
) -> Option<(String, &'static str)> {
    for (value, source) in [
        (lc_all, "LC_ALL"),
        (lc_messages, "LC_MESSAGES"),
        (lang, "LANG"),
    ] {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            return Some((value.to_owned(), source));
        }
    }
    None
}

use crate::model::{Detection, LocaleInfo};

/// Assemble the detection from the two halves. Either may be missing; both
/// missing is `unavailable`, never a made-up value.
pub fn build(
    process: Option<(String, &'static str)>,
    system: Option<String>,
    system_source: &str,
) -> Detection<LocaleInfo> {
    if process.is_none() && system.is_none() {
        return Detection::unavailable(
            "No locale variable is set and the system default could not be read",
        );
    }
    let (process_value, process_source) = match process {
        Some((value, source)) => (Some(value), Some(source.to_owned())),
        None => (None, None),
    };
    Detection::available_with_note(
        LocaleInfo {
            process: process_value,
            process_source,
            system: system.clone(),
            system_source: system.map(|_| system_source.to_owned()),
        },
        "process environment and system configuration",
        "The process locale is what this shell exported; the system locale is what the next login uses",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lc_all_wins_over_everything() {
        assert_eq!(
            process_locale(Some("C"), Some("de_DE.UTF-8"), Some("en_US.UTF-8")),
            Some(("C".to_owned(), "LC_ALL"))
        );
    }

    #[test]
    fn lc_messages_wins_over_lang() {
        assert_eq!(
            process_locale(None, Some("de_DE.UTF-8"), Some("en_US.UTF-8")),
            Some(("de_DE.UTF-8".to_owned(), "LC_MESSAGES"))
        );
    }

    #[test]
    fn an_empty_value_is_unset_not_a_locale() {
        assert_eq!(
            process_locale(Some(""), Some("  "), Some("en_US.UTF-8")),
            Some(("en_US.UTF-8".to_owned(), "LANG"))
        );
        assert_eq!(process_locale(Some(""), None, None), None);
    }

    #[test]
    fn either_half_alone_is_still_a_value() {
        let only_process = build(Some(("C".to_owned(), "LC_ALL")), None, "/etc/locale.conf");
        assert!(only_process.value().is_some());
        let only_system = build(None, Some("de_DE".to_owned()), "/etc/locale.conf");
        let value = only_system.value().expect("available");
        assert!(value.process.is_none());
        assert_eq!(value.system.as_deref(), Some("de_DE"));
        // Both missing is not a value.
        assert!(matches!(
            build(None, None, "/etc/locale.conf"),
            Detection::Unavailable { .. }
        ));
    }

    #[test]
    fn c_and_posix_are_locales_not_absence() {
        assert_eq!(
            process_locale(None, None, Some("C")),
            Some(("C".to_owned(), "LANG"))
        );
        assert_eq!(
            process_locale(None, None, Some("POSIX")),
            Some(("POSIX".to_owned(), "LANG"))
        );
    }
}
