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
        local_ip: if_selected(selection, ModuleId::LocalIp, local_ip),
    }
}
