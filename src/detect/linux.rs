//! Pure parsers, compiled on every supported platform for fixture tests.
use crate::model::*;
use std::collections::HashMap;

pub fn os_release(text: &str) -> Result<OsInfo, String> {
    let mut fields = HashMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (key, raw) = line
            .split_once('=')
            .ok_or("Invalid os-release assignment")?;
        if !matches!(key, "NAME" | "PRETTY_NAME" | "VERSION_ID" | "ID") {
            continue;
        }
        let raw = raw.trim();
        let value = if let Some(quote) = raw.chars().next().filter(|c| *c == '"' || *c == '\'') {
            if raw.len() < 2 || !raw.ends_with(quote) {
                return Err("Unterminated os-release quote".into());
            }
            let inner = &raw[1..raw.len() - 1];
            if quote == '\'' {
                inner.to_owned()
            } else {
                let mut value = String::new();
                let mut chars = inner.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == '\\'
                        && chars
                            .peek()
                            .is_some_and(|c| matches!(c, '"' | '\\' | '$' | '`'))
                    {
                        if let Some(escaped) = chars.next() {
                            value.push(escaped);
                        }
                    } else {
                        value.push(ch);
                    }
                }
                value
            }
        } else {
            if raw
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '\'' | '"'))
            {
                return Err("Unquoted os-release value contains spaces or quotes".into());
            }
            raw.to_owned()
        };
        fields.insert(key, value);
    }
    let name = fields
        .remove("NAME")
        .filter(|n| !n.trim().is_empty())
        .or_else(|| {
            fields
                .remove("PRETTY_NAME")
                .filter(|n| !n.trim().is_empty())
        })
        .ok_or("os-release has no nonempty NAME or PRETTY_NAME")?;
    Ok(OsInfo {
        family: OsFamily::Linux,
        name,
        version: fields.remove("VERSION_ID").filter(|s| !s.is_empty()),
        id: fields.remove("ID").filter(|s| !s.is_empty()),
    })
}

pub fn memory(text: &str, scope: Scope) -> Result<(MemoryInfo, bool), String> {
    let mut fields = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if !matches!(
            key,
            "MemTotal"
                | "MemAvailable"
                | "MemFree"
                | "Buffers"
                | "Cached"
                | "SReclaimable"
                | "Shmem"
        ) {
            continue;
        }
        let mut parts = value.split_whitespace();
        let number: u64 = parts
            .next()
            .ok_or("Missing memory value")?
            .parse()
            .map_err(|_| "Invalid memory value")?;
        if parts.next() != Some("kB") || parts.next().is_some() {
            return Err("Expected proc memory in kB (KiB)".into());
        }
        let bytes = number.checked_mul(1024).ok_or("Memory value overflow")?;
        fields.insert(key, bytes);
    }
    let total = *fields
        .get("MemTotal")
        .filter(|v| **v > 0)
        .ok_or("Missing or zero MemTotal")?;
    let (available, estimated, definition) = if let Some(available) = fields.get("MemAvailable") {
        (*available, false, MemoryDefinition::TotalMinusAvailable)
    } else {
        let mut available = 0_u64;
        for key in ["MemFree", "Buffers", "Cached"] {
            available = available
                .checked_add(
                    *fields
                        .get(key)
                        .ok_or("Insufficient memory fallback fields")?,
                )
                .ok_or("Memory sum overflow")?;
        }
        available = available
            .checked_add(*fields.get("SReclaimable").unwrap_or(&0))
            .ok_or("Memory sum overflow")?
            .checked_sub(*fields.get("Shmem").unwrap_or(&0))
            .ok_or("Inconsistent Shmem")?;
        (
            available,
            true,
            MemoryDefinition::TotalMinusFreeBuffersCache,
        )
    };
    let used_bytes = total
        .checked_sub(available)
        .ok_or("Available memory exceeds total")?;
    Ok((
        MemoryInfo {
            total_bytes: total,
            used_bytes,
            definition,
            scope,
        },
        estimated,
    ))
}

pub fn uptime(text: &str) -> Result<u64, String> {
    let value = text.split_whitespace().next().ok_or("Missing uptime")?;
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, "0"));
    if seconds.is_empty()
        || fraction.is_empty()
        || !seconds
            .bytes()
            .chain(fraction.bytes())
            .all(|c| c.is_ascii_digit())
    {
        return Err("Invalid uptime".into());
    }
    seconds.parse().map_err(|_| "Uptime overflow".into())
}

pub fn environment(release: &str, version: &str, hint: bool) -> Environment {
    let release = release.to_ascii_lowercase();
    let version = version.to_ascii_lowercase();
    if release.contains("wsl2") || release.contains("microsoft-standard") {
        Environment::Wsl { version: Some(2) }
    } else if release.contains("microsoft") || version.contains("microsoft") || hint {
        Environment::Wsl { version: None }
    } else {
        Environment::Native
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_quotes_and_no_shell_evaluation() {
        let os = os_release("# comment\nNAME=\"Example Linux\"\nVERSION_ID='42'\nID=example\nEXTRA=\"$(touch /bad)\"\n").unwrap();
        assert_eq!(os.name, "Example Linux");
        assert_eq!(os.version.as_deref(), Some("42"));
        assert_eq!(os.id.as_deref(), Some("example"));
        assert!(os_release("NAME=\"unterminated").is_err());
        assert!(os_release("ID=linux").is_err());
        assert!(os_release("NAME=\"\"").is_err());
    }

    #[test]
    fn memory_units_and_missing_available() {
        let (mem, estimate) = memory(
            "MemTotal: 1000 kB\nMemAvailable: 400 kB\n",
            Scope::OperatingSystem,
        )
        .unwrap();
        assert_eq!(mem.total_bytes, 1_024_000);
        assert_eq!(mem.used_bytes, 614_400);
        assert!(!estimate);
        let (mem, estimate) = memory("MemTotal: 1000 kB\nMemFree: 100 kB\nBuffers: 50 kB\nCached: 200 kB\nSReclaimable: 30 kB\nShmem: 20 kB\n", Scope::WslEnvironment).unwrap();
        assert_eq!(mem.used_bytes, 640 * 1024);
        assert_eq!(mem.scope, Scope::WslEnvironment);
        assert!(estimate);
    }

    #[test]
    fn memory_rejects_inconsistent_or_overflow_values() {
        for input in [
            "",
            "MemTotal: 0 kB",
            "MemTotal: 20 kB\nMemAvailable: 30 kB",
            "MemTotal: 1 MB",
            "MemTotal: 18446744073709551615 kB\nMemAvailable: 0 kB",
        ] {
            assert!(memory(input, Scope::OperatingSystem).is_err(), "{input}");
        }
    }

    #[test]
    fn uptime_is_seconds_not_milliseconds() {
        assert_eq!(uptime("1234.56 99.00\n").unwrap(), 1234);
        assert_eq!(uptime("0.01 0.00").unwrap(), 0);
        for text in ["", "nan 1", "-2.0 0", "inf 0", "18446744073709551616.0"] {
            assert!(uptime(text).is_err());
        }
    }

    #[test]
    fn wsl_requires_evidence_for_version() {
        assert_eq!(
            environment("6.6.87.2-microsoft-standard-WSL2", "", false),
            Environment::Wsl { version: Some(2) }
        );
        assert_eq!(
            environment("4.4.0-19041-Microsoft", "", false),
            Environment::Wsl { version: None }
        );
        assert_eq!(
            environment("custom", "", true),
            Environment::Wsl { version: None }
        );
        assert_eq!(
            environment("6.8.0-generic", "Linux version 6.8", false),
            Environment::Native
        );
    }
}
