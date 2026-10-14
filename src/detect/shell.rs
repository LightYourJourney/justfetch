use crate::model::*;

/// Match known executables only; arbitrary parent programs are not shells.
pub fn recognized_name(path: &str) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next()?.trim_start_matches('-');
    let lower = name.to_ascii_lowercase();
    let bare = lower.strip_suffix(".exe").unwrap_or(&lower);
    match bare {
        "sh" | "bash" | "zsh" | "fish" | "dash" | "ash" | "ksh" | "ksh93" | "tcsh" | "csh"
        | "nu" | "pwsh" | "powershell" | "cmd" | "xonsh" | "elvish" => Some(bare.to_owned()),
        _ => None,
    }
}

pub fn collect() -> Detection<ShellInfo> {
    let parent = crate::platform::parent_executable();
    if let Ok(exe) = &parent
        && let Some(name) = recognized_name(exe)
    {
        return Detection::available(
            ShellInfo {
                name,
                kind: ShellKind::CurrentParent,
            },
            "parent process executable",
        );
    }
    let variable = if cfg!(windows) { "ComSpec" } else { "SHELL" };
    match std::env::var(variable)
        .ok()
        .and_then(|p| recognized_name(&p))
    {
        Some(name) => Detection::estimated(
            ShellInfo {
                name,
                kind: ShellKind::ConfiguredDefault,
            },
            variable,
            format!(
                "Configured default may differ from current shell. Parent lookup: {}",
                parent
                    .as_ref()
                    .map(|_| "unrecognized executable".to_owned())
                    .unwrap_or_else(|error| error.to_string())
            ),
        ),
        None => match parent {
            Err(error) => {
                Detection::error("parent process executable", ErrorCode::ApiFailed, error)
            }
            Ok(_) => Detection::unavailable(
                "Immediate parent is not a recognized shell and no configured default is available",
            ),
        },
    }
}
