//! The glide console: a block-oriented terminal surface over the local
//! knowledge graph.
//!
//! Borrowed from Warp: each turn is an addressable block with a status and a
//! duration, the prompt stays pinned at the bottom, and blocks can be focused,
//! folded, copied, and re-run. Not borrowed: the PTY. This runs glide's verbs,
//! not your shell.

pub mod app;
pub mod block;
pub mod command;
pub mod engine;
pub mod input;
pub mod render;
pub mod run;
pub mod theme;

pub use app::{Action, App, Effect, Mode};
pub use block::{Block, BlockBody, BlockStatus};
pub use command::{parse as parse_command, Cmd};
pub use engine::{Engine, EngineCtx, GlideEngine};
pub use run::run_console;
