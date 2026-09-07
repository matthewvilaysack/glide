//! Optional LLM client for glide. Default-features = [] keeps the binary
//! pure-local (no reqwest, no tokio). Build with `--features anthropic` to
//! enable the Anthropic provider.

pub mod provider;
pub mod prompts;

#[cfg(feature = "anthropic")]
pub mod anthropic;

pub use provider::{Provider, ProviderId, PromptInput, ProviderError};

/// Resolve the configured provider. v1: returns `None` when no feature is
/// enabled; the CLI uses this to short-circuit `-p` into the keyword router.
pub fn resolve(_id: ProviderId) -> Option<Box<dyn Provider>> {
    #[cfg(feature = "anthropic")]
    {
        if matches!(_id, ProviderId::Anthropic) {
            return Some(Box::new(anthropic::AnthropicProvider::from_env()));
        }
    }
    None
}
