use super::{PlatformData, c_chars, uname};
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
        other => other.map(|_| unreachable!("available handled above")),
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
                error @ Detection::Error { .. } => return error.map(|_| unreachable!()),
                _ => {}
            }
        }
    }
    hostname.map(|name| HostInfo {
        name,
        kind: HostKind::Hostname,
    })
}

pub fn collect() -> PlatformData {
    let uname = uname();
    let hostname = match &uname {
        Ok(data) => Detection::available(c_chars(&data.nodename), "uname nodename"),
        Err(e) => Detection::error("uname", ErrorCode::ApiFailed, e),
    };
    let kernel = match &uname {
        Ok(data) => Detection::available(
            format!("{} {}", c_chars(&data.sysname), c_chars(&data.release)),
            "uname",
        ),
        Err(e) => Detection::error("uname", ErrorCode::ApiFailed, e),
    };
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
    let release = read("/etc/os-release");
    let release = if matches!(release, Detection::Unavailable { .. }) {
        read("/usr/lib/os-release")
    } else {
        release
    };
    let os = parsed(release, parse::os_release);
    let uptime = parsed(read("/proc/uptime"), parse::uptime).map(|seconds| UptimeInfo {
        seconds,
        scope: scope.clone(),
    });
    let memory = match parsed(read("/proc/meminfo"), |text| parse::memory(text, scope)) {
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
    };
    let host = host(hostname.clone(), &environment);
    // Do not query /mnt/c, Windows executables, DMI or host hardware under WSL.
    PlatformData {
        hostname,
        environment,
        os,
        host,
        kernel,
        uptime,
        memory,
    }
}
