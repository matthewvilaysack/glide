use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlideError {
    #[error("config error: {0}")]
    Config(String),

    #[error("graph not initialized; run `glide init`")]
    GraphNotInitialized,

    /// Its own variant rather than a Config error, because standing in the wrong
    /// directory is not a configuration problem and labelling it as one sends
    /// someone to look at a file that is fine.
    #[error("{0}")]
    NotInRepo(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("toml serialize: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("llm error: {0}")]
    Llm(String),

    #[error("{0}")]
    Other(String),
}

/// Maps a GlideError to a stable process exit code. Matches the plan:
/// 0 ok, 1 general, 2 config, 3 graph not init, 4 llm.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    General = 1,
    Config = 2,
    GraphNotInit = 3,
    Llm = 4,
}

impl ExitCode {
    pub fn from_error(err: &GlideError) -> ExitCode {
        match err {
            GlideError::Config(_) | GlideError::TomlParse(_) | GlideError::TomlSerialize(_) => {
                ExitCode::Config
            }
            GlideError::GraphNotInitialized => ExitCode::GraphNotInit,
            GlideError::Llm(_) => ExitCode::Llm,
            _ => ExitCode::General,
        }
    }
}

pub type Result<T> = std::result::Result<T, GlideError>;
