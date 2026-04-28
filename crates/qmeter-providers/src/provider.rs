use qmeter_core::types::{NormalizedError, NormalizedRow, ProviderId};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcquireContext {
    pub refresh: bool,
    pub debug: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderResult {
    pub rows: Vec<NormalizedRow>,
    pub errors: Vec<NormalizedError>,
    pub debug: Option<Value>,
}

pub trait Provider {
    fn id(&self) -> ProviderId;
    fn acquire(&self, ctx: AcquireContext) -> ProviderResult;
}

