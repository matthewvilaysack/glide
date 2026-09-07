use glide_common::Config;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModelsReport {
    pub enabled: bool,
    pub default_provider: String,
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub model: String,
    pub api_key_env: String,
    pub key_present: bool,
    pub feature_compiled: bool,
}

/// `anthropic_compiled` is passed in rather than read with `cfg!` here: this
/// crate has no such feature, so asking it would always answer false no matter
/// how the binary was built. The caller knows its own features.
pub fn models(cfg: &Config, anthropic_compiled: bool) -> ModelsReport {
    ModelsReport {
        enabled: cfg.llm.enabled,
        default_provider: cfg.llm.default_provider.clone(),
        providers: vec![ProviderInfo {
            id: "anthropic".into(),
            model: cfg.llm.anthropic.model.clone(),
            api_key_env: cfg.llm.anthropic.api_key_env.clone(),
            key_present: std::env::var_os(&cfg.llm.anthropic.api_key_env).is_some(),
            feature_compiled: anthropic_compiled,
        }],
    }
}
