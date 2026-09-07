//! One turn of the console. Every answer glide gives is a block: an input
//! line, a status, how long it took, and a body that wraps a `glide-core`
//! response verbatim. No new data model.

use glide_common::tools::RepomixStatus;
use glide_core::{
    DoctorReport, FrictionDigestResponse, FrictionLogResponse, ModelsReport, PlanResponse,
    RequestResponse, WhoOwnsResponse,
};
use glide_graph::index::IndexReport;
use glide_graph::GraphStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    Running,
    Ok,
    Failed,
    Notice,
}

#[derive(Debug, Clone)]
pub enum BlockBody {
    /// The verb is still running. Replaced in place when it returns.
    Pending,
    WhoOwns(Box<WhoOwnsResponse>),
    Plan(Box<PlanResponse>),
    Request(Box<RequestResponse>),
    Digest(Box<FrictionDigestResponse>),
    FrictionLogged(Box<FrictionLogResponse>),
    Doctor(Box<DoctorReport>),
    Stats(Box<GraphStats>),
    Indexed(Box<IndexReport>),
    Models(Box<ModelsReport>),
    Tools(Box<RepomixStatus>),
    /// `config list` — the effective config, already rendered.
    Toml(String),
    /// The router matched nothing. Carries what to try instead.
    Unrouted {
        reason: String,
        suggestions: Vec<String>,
    },
    /// Informational: the welcome block, `help`, the "run glide init" hint.
    Notice {
        title: String,
        lines: Vec<String>,
    },
    Error {
        message: String,
    },
}

impl BlockBody {
    pub fn status(&self) -> BlockStatus {
        match self {
            BlockBody::Pending => BlockStatus::Running,
            BlockBody::Error { .. } => BlockStatus::Failed,
            BlockBody::Notice { .. } | BlockBody::Unrouted { .. } => BlockStatus::Notice,
            // A doctor run that found problems is a failing block, not an
            // error: the report is exactly what the hire needs to read.
            BlockBody::Doctor(r) if r.failures() > 0 => BlockStatus::Failed,
            _ => BlockStatus::Ok,
        }
    }

    /// Short label shown next to the input line, so a collapsed block still
    /// says what it is.
    pub fn kind(&self) -> &'static str {
        match self {
            BlockBody::Pending => "running",
            BlockBody::WhoOwns(_) => "who-owns",
            BlockBody::Plan(_) => "plan",
            BlockBody::Request(_) => "request",
            BlockBody::Digest(_) => "friction",
            BlockBody::FrictionLogged(_) => "friction log",
            BlockBody::Doctor(_) => "doctor",
            BlockBody::Stats(_) => "index",
            BlockBody::Indexed(_) => "index build",
            BlockBody::Models(_) => "models",
            BlockBody::Tools(_) => "tools",
            BlockBody::Toml(_) => "config",
            BlockBody::Unrouted { .. } => "unrouted",
            BlockBody::Notice { .. } => "notice",
            BlockBody::Error { .. } => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: usize,
    /// What the hire typed. Empty for the welcome block.
    pub input: String,
    pub body: BlockBody,
    pub elapsed_ms: u128,
    pub collapsed: bool,
}

impl Block {
    pub fn new(id: usize, input: impl Into<String>, body: BlockBody, elapsed_ms: u128) -> Self {
        Block {
            id,
            input: input.into(),
            body,
            elapsed_ms,
            collapsed: false,
        }
    }

    pub fn status(&self) -> BlockStatus {
        self.body.status()
    }
}
