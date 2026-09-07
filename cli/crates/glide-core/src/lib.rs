//! Verb implementations for the glide CLI. Each function returns structured
//! data; the CLI crate is responsible for formatting and exit codes.

pub mod doctor;
pub mod friction;
pub mod models;
pub mod plan;
pub mod request;
pub mod router;
pub mod who_owns;

pub use doctor::{doctor, Check, CheckState, DoctorReport};
pub use friction::{digest, log_event, FrictionDigestResponse, FrictionLogResponse};
pub use models::{models, ModelsReport, ProviderInfo};
pub use plan::{plan, PlanResponse};
pub use request::{request, RequestResponse};
pub use router::{route_one_shot, RouteDecision};
pub use who_owns::{who_owns, WhoOwnsResponse};
