use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Estimated,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Detection<T> {
    Available {
        value: T,
        source: String,
        confidence: Confidence,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    Unavailable {
        reason: String,
    },
    Unsupported {
        reason: String,
    },
    Error {
        source: String,
        code: ErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Io,
    InvalidData,
    ApiFailed,
}

impl<T> Detection<T> {
    pub fn available(value: T, source: impl Into<String>) -> Self {
        Self::Available {
            value,
            source: source.into(),
            confidence: Confidence::Confirmed,
            note: None,
        }
    }

    /// A measured value with a caveat that is not about its accuracy. Marking
    /// such a value estimated would tell a consumer the numbers themselves are
    /// approximate, which is a different claim.
    pub fn available_with_note(
        value: T,
        source: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self::Available {
            value,
            source: source.into(),
            confidence: Confidence::Confirmed,
            note: Some(note.into()),
        }
    }

    pub fn estimated(value: T, source: impl Into<String>, note: impl Into<String>) -> Self {
        Self::Available {
            value,
            source: source.into(),
            confidence: Confidence::Estimated,
            note: Some(note.into()),
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }

    /// No route this project is willing to take reaches this value on that
    /// platform, which is different from a value that is unavailable right now.
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported {
            reason: reason.into(),
        }
    }

    pub fn error(source: impl Into<String>, code: ErrorCode, message: impl ToString) -> Self {
        Self::Error {
            source: source.into(),
            code,
            message: message.to_string(),
        }
    }

    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Available { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Detection<U> {
        match self {
            Self::Available {
                value,
                source,
                confidence,
                note,
            } => Detection::Available {
                value: f(value),
                source,
                confidence,
                note,
            },
            Self::Unavailable { reason } => Detection::Unavailable { reason },
            Self::Unsupported { reason } => Detection::Unsupported { reason },
            Self::Error {
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
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub schema_version: u32,
    pub identity: Identity,
    pub environment: Environment,
    pub os: Detection<OsInfo>,
    pub host: Detection<HostInfo>,
    pub kernel: Detection<String>,
    pub uptime: Detection<UptimeInfo>,
    pub cpu: Detection<CpuInfo>,
    pub memory: Detection<MemoryInfo>,
    pub swap: Detection<SwapInfo>,
    pub disk: Detection<DiskInfo>,
    pub shell: Detection<ShellInfo>,
    pub terminal: Detection<TerminalInfo>,
    pub local_ip: Detection<LocalIpInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Identity {
    pub username: Detection<String>,
    pub hostname: Detection<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Environment {
    #[default]
    Native,
    Wsl {
        version: Option<u8>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OsInfo {
    pub family: OsFamily,
    pub name: String,
    pub version: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HostInfo {
    pub name: String,
    pub kind: HostKind,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    DeviceModel,
    Hostname,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UptimeInfo {
    pub seconds: u64,
    pub scope: Scope,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    OperatingSystem,
    WslEnvironment,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CpuInfo {
    pub name: String,
    pub logical_cores: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub definition: MemoryDefinition,
    pub scope: Scope,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDefinition {
    TotalMinusAvailable,
    TotalMinusFreeBuffersCache,
    AppWiredCompressed,
}

/// Swap is reported separately from memory because a system can have none, and
/// zero is an answer rather than a missing one.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SwapInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

/// The filesystem containing `path`, named by the path that was queried rather
/// than by a mount point the platform may not expose. `used_bytes` is
/// `total - free` and `available_bytes` is what the calling user may still
/// allocate: the two coincide on a filesystem with no reserved blocks and
/// differ by the reserve on one with them.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiskInfo {
    pub path: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TerminalInfo {
    pub name: String,
    pub version: Option<String>,
}

/// Every routable IPv4 address on an interface that is up. Listing all of them
/// rather than picking one avoids an arbitrary choice: on a machine running
/// containers, whichever interface the kernel happens to report first is often
/// a bridge, not the one carrying traffic.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocalIpInfo {
    pub addresses: Vec<InterfaceAddress>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InterfaceAddress {
    /// Absent when the platform reports an address on an adapter it cannot
    /// name. An empty string would render as `()`, which reads like a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    pub address: std::net::Ipv4Addr,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShellInfo {
    pub name: String,
    pub kind: ShellKind,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    CurrentParent,
    ConfiguredDefault,
}
