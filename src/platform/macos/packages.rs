//! Homebrew's installed formulae and casks.

use super::*;

/// Homebrew's installed formulae and casks. The count is directories in each
/// prefix's Cellar and Caskroom: a formula with several versions installed is
/// still one formula. Homebrew's own database is not parsed, so this does not
/// claim to match `brew list` exactly.
pub(super) fn packages() -> Detection<PackagesInfo> {
    // `None` means the directory is absent; `Err` means it exists and could not
    // be read, which is not evidence that Homebrew is missing.
    let count_dirs = |path: &std::path::Path| -> Result<Option<u64>, std::io::Error> {
        match std::fs::read_dir(path) {
            Ok(entries) => Ok(Some(
                entries
                    .flatten()
                    .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
                    .count() as u64,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    };
    let mut unreadable: Option<String> = None;
    let mut managers = Vec::new();
    for prefix in ["/opt/homebrew", "/usr/local"] {
        let base = std::path::Path::new(prefix);
        let cellar = match count_dirs(&base.join("Cellar")) {
            Ok(cellar) => cellar,
            Err(error) => {
                unreadable.get_or_insert(format!("{prefix}/Cellar could not be read: {error}"));
                continue;
            }
        };
        if let Some(count) = cellar.filter(|count| *count > 0) {
            managers.push(PackageManager {
                name: "homebrew formulae".to_owned(),
                count,
                definition: format!("directories in {prefix}/Cellar"),
            });
            if let Ok(Some(casks)) = count_dirs(&base.join("Caskroom")) {
                managers.push(PackageManager {
                    name: "homebrew casks".to_owned(),
                    count: casks,
                    definition: format!("directories in {prefix}/Caskroom"),
                });
            }
            // The other prefix is a second installation of the same manager;
            // counting it too would merge two independent sets silently.
            break;
        }
    }
    if managers.is_empty() {
        if let Some(reason) = unreadable {
            return Detection::unavailable(reason);
        }
        return Detection::unsupported(
            "No Homebrew Cellar was found in /opt/homebrew or /usr/local",
        );
    }
    Detection::available(
        PackagesInfo { managers },
        "Homebrew Cellar and Caskroom directories",
    )
}
