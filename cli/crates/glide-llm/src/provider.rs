use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    Anthropic,
}

#[derive(Debug, Serialize)]
pub struct PromptInput<'a> {
    pub system: &'a str,
    pub user: &'a str,
    pub max_tokens: u32,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("missing api key in env: {0}")]
    MissingKey(String),
    #[error("network: {0}")]
    Network(String),
    #[error("api: {0}")]
    Api(String),
}

pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn complete(&self, input: PromptInput<'_>) -> Result<String, ProviderError>;
}
