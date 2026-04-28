use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Structured,
    Parsed,
    Cache,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
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
