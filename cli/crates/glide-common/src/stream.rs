use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::errors::GlideError;

/// Output stream mode. Mirrors NCA's `--stream`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Stream {
    #[default]
    Human,
    Ndjson,
    Off,
}

impl FromStr for Stream {
    type Err = GlideError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "human" => Ok(Stream::Human),
            "ndjson" => Ok(Stream::Ndjson),
            "off" => Ok(Stream::Off),
            other => Err(GlideError::Config(format!(
                "invalid stream mode: {other} (want human|ndjson|off)"
            ))),
        }
    }
}

impl std::fmt::Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Stream::Human => "human",
            Stream::Ndjson => "ndjson",
            Stream::Off => "off",
        })
    }
}
