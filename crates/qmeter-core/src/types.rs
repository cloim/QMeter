use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Claude,
    Codex,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Structured,
    Parsed,
    Cache,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Structured => "structured",
            Self::Parsed => "parsed",
            Self::Cache => "cache",
        }
    }

    pub fn is_cache(self) -> bool {
        matches!(self, Self::Cache)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedErrorType {
    NotInstalled,
    AuthRequired,
    Offline,
    TtyUnavailable,
    Timeout,
    ParseFailed,
    InvalidResponse,
    AcquireFailed,
    Unexpected,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRow {
    pub provider: ProviderId,
    pub window: String,
    pub used: Option<u64>,
    pub limit: Option<u64>,
    pub used_percent: Option<f64>,
    pub reset_at: Option<String>,
    pub source: SourceKind,
    pub confidence: Confidence,
    pub stale: bool,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedError {
    pub provider: ProviderId,
    #[serde(rename = "type")]
    pub error_type: NormalizedErrorType,
    pub message: String,
    pub actionable: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSnapshot {
    pub fetched_at: String,
    pub rows: Vec<NormalizedRow>,
    pub errors: Vec<NormalizedError>,
}
