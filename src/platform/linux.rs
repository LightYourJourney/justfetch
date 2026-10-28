use super::{PlatformData, c_chars, disk, local_ip, uname, uname_identity};
use crate::module::{ModuleId, if_selected, wanted};
use crate::{detect::linux as parse, model::*};
use std::{
    fs::File,
    io::{self, Read},
};

pub fn parent_executable() -> io::Result<String> {
    // SAFETY: getppid has no pointer arguments and only reads process metadata.
    let pid = unsafe { libc::getppid() };
    let path = std::fs::read_link(format!("/proc/{pid}/exe"))?;
    path.into_os_string()
        .into_string()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Parent executable is not UTF-8"))
}

fn read(path: &str) -> Detection<String> {
    let result = (|| -> io::Result<String> {
        let mut text = String::new();
        File::open(path)?.take(65_537).read_to_string(&mut text)?;
        if text.len() > 65_536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "System file exceeds 64 KiB",
            ));
        }
        Ok(text)
    })();
    match result {
        Ok(text) => Detection::available(text, path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Detection::unavailable(format!("{path} is absent"))
        }
        Err(e) => Detection::error(path, ErrorCode::Io, e),
    }
}

fn parsed<T>(
    data: Detection<String>,
    parser: impl FnOnce(&str) -> Result<T, String>,
) -> Detection<T> {
    match data {
        Detection::Available { value, source, .. } => match parser(&value) {
            Ok(value) => Detection::available(value, source),
            Err(message) => Detection::error(source, ErrorCode::InvalidData, message),
        },
        Detection::Unavailable { reason } => Detection::Unavailable { reason },
        Detection::Unsupported { reason } => Detection::Unsupported { reason },
        Detection::Redacted => Detection::Redacted,
        Detection::Deselected => Detection::Deselected,
        Detection::Error {
            source,
            code,
            message,
        } => Detection::Error {
            source,
            code,
            message,
        },
    }
}

fn host(hostname: Detection<String>, environment: &Environment) -> Detection<HostInfo> {
    if matches!(environment, Environment::Native) {
        for path in [
            "/sys/devices/virtual/dmi/id/product_name",
            "/sys/firmware/devicetree/base/model",
        ] {
            match read(path) {
                Detection::Available { value, source, .. } => {
                    let name = value.trim_matches(['\0', '\n', '\r', ' ']);
                    if !name.is_empty()
                        && ![
                            "To Be Filled By O.E.M.",
                            "System Product Name",
                            "Default string",
                        ]
                        .contains(&name)
                    {
                        return Detection::available(
                            HostInfo {
                                name: name.to_owned(),
                                kind: HostKind::DeviceModel,
                            },
                            source,
                        );
                    }
                }
                // A model source that exists but cannot be read is this field's
                // answer; only an absent one falls through to the hostname.
                Detection::Error {
                    source,
                    code,
                    message,
                } => {
                    return Detection::Error {
                        source,
                        code,
                        message,
                    };
                }
                _ => {}
            }
        }
    }
    hostname.map(|name| HostInfo {
        name,
        kind: HostKind::Hostname,
    })
}

/// Batteries and the AC adapter, from sysfs. This reads one fixed system
/// directory and nothing else; a container or a desktop simply has no
/// `Battery` entries there.
fn battery() -> Detection<BatteryInfo> {
    let root = std::path::Path::new("/sys/class/power_supply");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return Detection::unavailable(format!("{} cannot be read: {error}", root.display()));
        }
    };
    let text = |path: &std::path::Path| {
        std::fs::read_to_string(path)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let mut units = Vec::new();
    let mut ac_online = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = text(&path.join("type")).unwrap_or_default();
        match kind.as_str() {
            "Battery" => {
                // `scope` separates the machine's battery from a mouse's or a
                // headset's; `present` is 0 for an empty bay.
                if text(&path.join("scope")).is_some_and(|scope| scope != "System") {
                    continue;
                }
                if text(&path.join("present")).as_deref() == Some("0") {
                    continue;
                }
                let state = match text(&path.join("status")).as_deref() {
                    Some("Charging") => BatteryState::Charging,
                    Some("Discharging") => BatteryState::Discharging,
                    Some("Full") => BatteryState::Full,
                    _ => BatteryState::Unknown,
                };
                units.push(BatteryUnit {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    // A unit with no capacity file reports `None`, not zero; a
                    // value above 100 is not a percentage, so it is not one.
                    percent: text(&path.join("capacity"))
                        .and_then(|v| v.parse::<u8>().ok())
                        .filter(|percent| *percent <= 100),
                    state,
                });
            }
            // The adapter, not a battery: it only says whether mains is live.
            "Mains" | "USB" | "USB_C" | "USB_PD" | "USB_TYPE_C" | "AC" | "ACAD" => {
                if let Some(online) = text(&path.join("online")) {
                    ac_online = Some(online == "1");
                }
            }
            _ => {}
        }
    }
    if units.is_empty() {
        return Detection::unsupported(
            "No battery is present; sysfs lists no power supply of type Battery",
        );
    }
    let power = match ac_online {
        Some(true) => PowerSource::Ac,
        Some(false) => PowerSource::Battery,
        None => {
            let charging = units
                .iter()
                .any(|unit| matches!(unit.state, BatteryState::Charging | BatteryState::Full));
            if charging {
                PowerSource::Ac
            } else if units
                .iter()
                .all(|unit| unit.state == BatteryState::Discharging)
            {
                PowerSource::Battery
            } else {
                PowerSource::Unknown
            }
        }
    };
    Detection::available(BatteryInfo { units, power }, "/sys/class/power_supply")
}

/// Graphics adapters from DRM sysfs. Names are only reported when the kernel
/// or the driver exposes one: PCI ids without a name database are reported as
/// ids, because inventing a marketing name from a numeric table is a guess.
fn gpu() -> Detection<GpuInfo> {
    let root = std::path::Path::new("/sys/class/drm");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return Detection::unavailable(format!("{} cannot be read: {error}", root.display()));
        }
    };
    let mut devices = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // `card0` is an adapter; `card0-DP-1` is a connector on it.
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let Ok(uevent) = std::fs::read_to_string(entry.path().join("device/uevent")) else {
            // A card with no device node is a virtual framebuffer or a
            // connector-only stub; it is not evidence of an adapter.
            continue;
        };
        let field = |key: &str| {
            uevent
                .lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let pci_id = field("PCI_ID");
        let driver = field("DRIVER");
        let vendor = pci_id
            .as_deref()
            .and_then(crate::detect::hardware::pci_vendor);
        // NVIDIA publishes a model name in procfs; others do not, and the
        // sysfs PCI id is not a name.
        let display_name = field("PCI_SLOT_NAME")
            .and_then(|slot| {
                std::fs::read_to_string(format!("/proc/driver/nvidia/gpus/{slot}/information")).ok()
            })
            .and_then(|info| crate::detect::hardware::nvidia_model(&info));
        devices.push(GpuDevice {
            name: display_name,
            vendor: vendor.map(str::to_owned),
            id: pci_id,
            driver,
            // Integrated versus discrete cannot be told from these files.
            kind: GpuKind::Unknown,
        });
    }
    if devices.is_empty() {
        return Detection::unsupported("No graphics adapter is present in /sys/class/drm");
    }
    Detection::available(GpuInfo { devices }, "/sys/class/drm")
}

/// Displays from DRM connectors. sysfs knows which connectors are connected
/// and which modes they advertise, but not which mode is in use: that lives in
/// the compositor. The preferred mode is reported as an estimate with that
/// caveat rather than presented as the current resolution.
fn display() -> Detection<DisplayInfo> {
    let root = std::path::Path::new("/sys/class/drm");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return Detection::unavailable(format!("{} cannot be read: {error}", root.display()));
        }
    };
    let mut displays = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Connectors are named `card0-eDP-1`; `card0` is the adapter itself.
        if !name.contains('-') {
            continue;
        }
        let status = std::fs::read_to_string(entry.path().join("status")).unwrap_or_default();
        if status.trim() != "connected" {
            continue;
        }
        let modes = std::fs::read_to_string(entry.path().join("modes")).unwrap_or_default();
        // The first line is the connector's preferred mode.
        let Some((width, height)) = modes.lines().next().and_then(|mode| {
            let (w, h) = mode.trim().split_once('x')?;
            Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))
        }) else {
            continue;
        };
        let kind = if name.contains("eDP") || name.contains("LVDS") {
            DisplayKind::BuiltIn
        } else {
            DisplayKind::Unknown
        };
        displays.push(DisplayUnit {
            name: Some(name),
            width,
            height,
            refresh_hz: None,
            // sysfs has no primary bit.
            primary: None,
            kind,
        });
    }
    if displays.is_empty() {
        return Detection::unsupported("No display is connected to this machine");
    }
    Detection::estimated(
        DisplayInfo {
            displays,
            definition: "each connector's preferred mode; the mode in use is set by the compositor"
                .to_owned(),
        },
        "/sys/class/drm",
        "Each connector's preferred mode; the mode actually in use is set by the compositor",
    )
}

/// Installed counts from the package databases this build can read without a
/// process or a database driver. RPM is not read: its database is SQLite or
/// Berkeley DB, and guessing from file counts would be a number about nothing.
fn packages() -> Detection<PackagesInfo> {
    let mut managers = Vec::new();
    // dpkg's status file is authoritative: a `.list` file also survives for a
    // package that was removed but not purged, and there is one per
    // architecture, so counting those overcounts what is installed.
    if let Ok(status) = std::fs::read_to_string("/var/lib/dpkg/status") {
        let count = crate::detect::hardware::dpkg_installed(&status);
        if count > 0 {
            managers.push(PackageManager {
                name: "dpkg".to_owned(),
                count,
                definition: "installed packages: entries in /var/lib/dpkg/status with status install ok installed".to_owned(),
            });
        }
    }
    // pacman keeps one directory per package under `local`.
    if let Ok(entries) = std::fs::read_dir("/var/lib/pacman/local") {
        let count = entries
            .flatten()
            .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
            .count() as u64;
        if count > 0 {
            managers.push(PackageManager {
                name: "pacman".to_owned(),
                count,
                definition: "installed packages: one directory in /var/lib/pacman/local".to_owned(),
            });
        }
    }
    if managers.is_empty() {
        let reason = if std::path::Path::new("/nix/store").exists() {
            "Nix is present, but a store-path count is not a package count and this build does not read profiles"
        } else if std::path::Path::new("/var/lib/rpm").exists() {
            "An RPM database is present, but this build has no RPM database reader"
        } else {
            "No dpkg, pacman, RPM or Nix database was found"
        };
        return Detection::unsupported(reason);
    }
    Detection::available(PackagesInfo { managers }, "package database directories")
}

/// Gather the session evidence and let the pure detector decide. The platform
/// adapter only reads the environment; what the variables mean is in
/// `detect::desktop`, where it can be tested without a desktop.
fn desktop() -> Detection<DesktopInfo> {
    use crate::detect::desktop::{Evidence, describe};
    let var = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    };
    let current = var("XDG_CURRENT_DESKTOP");
    let session = var("XDG_SESSION_DESKTOP");
    let desktop = var("DESKTOP_SESSION");
    let session_type = var("XDG_SESSION_TYPE");
    describe(&Evidence {
        xdg_current: current.as_deref(),
        xdg_session: session.as_deref(),
        desktop_session: desktop.as_deref(),
        sway_socket: var("SWAYSOCK").is_some(),
        hyprland_signature: var("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        session_type: session_type.as_deref(),
        wayland_display: var("WAYLAND_DISPLAY").is_some(),
        display: var("DISPLAY").is_some(),
    })
}

/// The process locale from the POSIX variables, and the system default from
/// the file the distribution maintains for it. The two are different things and
/// are reported separately.
fn locale() -> Detection<LocaleInfo> {
    let var = |name: &str| std::env::var(name).ok();
    let lc_all = var("LC_ALL");
    let lc_messages = var("LC_MESSAGES");
    let lang = var("LANG");
    let process = crate::detect::locale::process_locale(
        lc_all.as_deref(),
        lc_messages.as_deref(),
        lang.as_deref(),
    );
    // systemd distributions keep it in /etc/locale.conf; Debian's own file is
    // the fallback. A missing file means the platform has no separate default.
    let (system, source) = ["/etc/locale.conf", "/etc/default/locale"]
        .into_iter()
        .find_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let value = text
                .lines()
                .find_map(|line| line.trim().strip_prefix("LANG="))?
                .trim()
                .trim_matches('"')
                .to_owned();
            (!value.is_empty()).then_some((value, path))
        })
        .map_or((None, "/etc/locale.conf"), |(value, path)| {
            (Some(value), path)
        });
    crate::detect::locale::build(process, system, source)
}

pub fn collect(selection: &[ModuleId]) -> PlatformData {
    let uname = uname();
    // The hostname is needed for the title even when no module asks for it.
    let (hostname, kernel) = uname_identity(&uname);
    let environment = match &uname {
        Ok(data) => parse::environment(
            &c_chars(&data.release),
            &c_chars(&data.version),
            std::env::var_os("WSL_INTEROP").is_some()
                || std::env::var_os("WSL_DISTRO_NAME").is_some(),
        ),
        Err(_) => parse::environment("", "", std::env::var_os("WSL_DISTRO_NAME").is_some()),
    };
    let scope = if matches!(environment, Environment::Wsl { .. }) {
        Scope::WslEnvironment
    } else {
        Scope::OperatingSystem
    };
    // /etc takes precedence. Fall back only when absent, never on malformed data.
    let os = if_selected(selection, ModuleId::Os, || {
        let release = read("/etc/os-release");
        let release = if matches!(release, Detection::Unavailable { .. }) {
            read("/usr/lib/os-release")
        } else {
            release
        };
        parsed(release, parse::os_release)
    });
    let uptime = if_selected(selection, ModuleId::Uptime, || {
        parsed(read("/proc/uptime"), parse::uptime).map(|seconds| UptimeInfo {
            seconds,
            scope: scope.clone(),
        })
    });
    // /proc/meminfo carries both fields, so it is read when either is wanted.
    let (memory, swap) = if wanted(selection, ModuleId::Memory) || wanted(selection, ModuleId::Swap)
    {
        let meminfo = read("/proc/meminfo");
        let swap = if_selected(selection, ModuleId::Swap, || {
            parsed(meminfo.clone(), parse::swap)
        });
        let memory = if_selected(selection, ModuleId::Memory, || {
            match parsed(meminfo, |text| parse::memory(text, scope)) {
                Detection::Available {
                    value: (value, true),
                    source,
                    ..
                } => Detection::estimated(
                    value,
                    source,
                    "MemAvailable is absent; estimated from free, buffers and reclaimable cache",
                ),
                other => other.map(|(value, _)| value),
            }
        });
        (memory, swap)
    } else {
        (Detection::deselected(), Detection::deselected())
    };
    let host = if_selected(selection, ModuleId::Host, || {
        host(hostname.clone(), &environment)
    });
    // Do not query /mnt/c, Windows executables, DMI or host hardware under WSL.
    PlatformData {
        hostname,
        environment,
        os,
        host,
        kernel: if_selected(selection, ModuleId::Kernel, || kernel),
        uptime,
        memory,
        swap,
        disk: if_selected(selection, ModuleId::Disk, disk),
        gpu: if_selected(selection, ModuleId::Gpu, gpu),
        display: if_selected(selection, ModuleId::Display, display),
        packages: if_selected(selection, ModuleId::Packages, packages),
        desktop: if_selected(selection, ModuleId::Desktop, desktop),
        locale: if_selected(selection, ModuleId::Locale, locale),
        battery: if_selected(selection, ModuleId::Battery, battery),
        local_ip: if_selected(selection, ModuleId::LocalIp, local_ip),
    }
}
