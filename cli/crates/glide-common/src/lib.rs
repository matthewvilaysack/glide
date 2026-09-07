//! Shared types, config, errors, and path helpers for the glide CLI.

pub mod config;
pub mod config_edit;
pub mod errors;
pub mod paths;
pub mod stream;
pub mod tools;

pub use config::Config;
pub use errors::{ExitCode, GlideError};
pub use stream::Stream;
