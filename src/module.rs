//! The stable identifiers a user can name in `--modules` or a config file.
//!
//! The enum is the one place a module is named. Selection, ordering, labels,
//! collection and rendering all key off it, so adding a module means adding a
//! variant and the arms the compiler then demands — not editing a list of
//! indices in several files.

/// One system-information module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleId {
    Os,
    Host,
    Kernel,
    Uptime,
    Cpu,
    Gpu,
    Memory,
    Swap,
    Disk,
    Battery,
    Shell,
    Terminal,
    LocalIp,
}

impl ModuleId {
    /// Every module, in the order a default run shows them.
    pub const ALL: [ModuleId; 13] = [
        ModuleId::Os,
        ModuleId::Host,
        ModuleId::Kernel,
        ModuleId::Uptime,
        ModuleId::Cpu,
        ModuleId::Gpu,
        ModuleId::Memory,
        ModuleId::Swap,
        ModuleId::Disk,
        ModuleId::Battery,
        ModuleId::Shell,
        ModuleId::Terminal,
        ModuleId::LocalIp,
    ];

    /// The identifier used in `--modules`, config files and diagnostics.
    pub fn id(self) -> &'static str {
        match self {
            ModuleId::Os => "os",
            ModuleId::Host => "host",
            ModuleId::Kernel => "kernel",
            ModuleId::Uptime => "uptime",
            ModuleId::Cpu => "cpu",
            ModuleId::Gpu => "gpu",
            ModuleId::Memory => "memory",
            ModuleId::Swap => "swap",
            ModuleId::Disk => "disk",
            ModuleId::Battery => "battery",
            ModuleId::Shell => "shell",
            ModuleId::Terminal => "terminal",
            ModuleId::LocalIp => "local_ip",
        }
    }

    /// The label shown when the user has not set one. Six columns is the
    /// widest, which is what the default layout was built around.
    pub fn label(self) -> &'static str {
        match self {
            ModuleId::Os => "OS",
            ModuleId::Host => "Host",
            ModuleId::Kernel => "Kernel",
            ModuleId::Uptime => "Uptime",
            ModuleId::Cpu => "CPU",
            ModuleId::Gpu => "GPU",
            ModuleId::Memory => "Memory",
            ModuleId::Swap => "Swap",
            ModuleId::Disk => "Disk",
            ModuleId::Battery => "Battery",
            ModuleId::Shell => "Shell",
            ModuleId::Terminal => "Term",
            ModuleId::LocalIp => "IP",
        }
    }

    /// Parse an identifier, case-insensitively. `local-ip` is accepted because
    /// a hyphen reads better in some configs and the underscore is easy to lose.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        let text = if text == "local-ip" {
            "local_ip"
        } else {
            &text
        };
        ModuleId::ALL.into_iter().find(|module| module.id() == text)
    }
}

/// Collect only when the module is selected, and report `deselected` otherwise.
/// The gate lives here so every collector states the same rule the same way.
pub fn if_selected<T>(
    selection: &[ModuleId],
    id: ModuleId,
    collect: impl FnOnce() -> crate::model::Detection<T>,
) -> crate::model::Detection<T> {
    if selection.contains(&id) {
        collect()
    } else {
        crate::model::Detection::deselected()
    }
}

/// Whether `modules` asks for `id`. A linear scan over at most eleven entries
/// is cheaper than any lookup structure and keeps the call sites plain.
pub fn wanted(modules: &[ModuleId], id: ModuleId) -> bool {
    modules.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_round_trip_and_are_unique() {
        for module in ModuleId::ALL {
            assert_eq!(ModuleId::parse(module.id()), Some(module));
            assert_eq!(
                ModuleId::parse(&module.id().to_ascii_uppercase()),
                Some(module)
            );
        }
        let mut ids: Vec<&str> = ModuleId::ALL.iter().map(|m| m.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ModuleId::ALL.len());
    }

    #[test]
    fn unknown_and_malformed_identifiers_are_rejected() {
        for text in ["", "cpu ", "bogus", "os,cpu", "lcoal_ip"] {
            let parsed = ModuleId::parse(text);
            // `"cpu "` trims to a valid id; everything else must fail.
            if text.trim().eq_ignore_ascii_case("cpu") {
                assert_eq!(parsed, Some(ModuleId::Cpu));
            } else {
                assert_eq!(parsed, None, "{text}");
            }
        }
        assert_eq!(ModuleId::parse("local-ip"), Some(ModuleId::LocalIp));
    }
}
