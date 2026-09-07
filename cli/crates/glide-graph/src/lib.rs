//! On-device SQLite knowledge graph for glide.

pub mod index;
pub mod queries;
pub mod schema;

pub use queries::{GraphStats, OwnerHit, OwnerKind};
pub use schema::{init_db, open, GraphDb};
