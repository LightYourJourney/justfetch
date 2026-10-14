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
    pub shell: Detection<ShellInfo>,
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
