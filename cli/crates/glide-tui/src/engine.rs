//! Turns a typed line into a block body. The console owns no product logic: it
//! parses verbs with `command::parse`, falls back to
//! `glide_core::route_one_shot` for questions, and calls the same functions the
//! CLI calls.

use std::path::PathBuf;

use glide_common::paths::workspace_dir;
use glide_common::Config;
use glide_core::RouteDecision;
use glide_graph::GraphDb;

use crate::block::BlockBody;
use crate::command::{self, Cmd};

/// Seam between the console and glide's verbs. Lets `App::update` be tested
/// without a repo, a database, or a terminal.
pub trait Engine {
    fn run(&mut self, input: &str) -> BlockBody;

    /// Counts for the top bar. Re-read after every block, because verbs like
    /// `index build` and `friction log` change them and a stale header is a
    /// lie the hire has no way to spot.
    fn stats_label(&self) -> Option<String> {
        None
    }
}

/// Everything the verbs need besides the graph handle.
pub struct EngineCtx {
    pub repo_root: PathBuf,
    pub db_path: PathBuf,
    pub config: Config,
    /// Whether the *binary* was built with the anthropic feature. Asking
    /// `cfg!` from this crate would always answer false.
    pub anthropic_compiled: bool,
    /// Mirrors the CLI's `--safe`: no writes, no network.
    pub safe: bool,
}

pub struct GlideEngine {
    /// `None` when the repo has no `.glide/` yet. Every graph verb then answers
    /// with the init hint instead of failing.
    pub db: Option<GraphDb>,
    pub ctx: EngineCtx,
    pub top: usize,
}

impl GlideEngine {
    pub fn new(db: Option<GraphDb>, ctx: EngineCtx) -> Self {
        GlideEngine { db, ctx, top: 5 }
    }

    fn needs_init() -> BlockBody {
        BlockBody::Notice {
            title: "no graph in this repo yet".into(),
            lines: vec![
                "Run `glide init`, then `index build`.".into(),
                "The console reads the same SQLite graph the verbs do.".into(),
            ],
        }
    }

    fn refused(what: &str) -> BlockBody {
        BlockBody::Notice {
            title: format!("{what} needs writes, and this session is --safe"),
            lines: vec!["Restart without `--safe` to allow it.".into()],
        }
    }

    fn err(e: impl std::fmt::Display) -> BlockBody {
        BlockBody::Error {
            message: format!("{e:#}"),
        }
    }

    fn config_local(&self) -> PathBuf {
        workspace_dir(&self.ctx.repo_root).join("config.local.toml")
    }

    fn run_command(&mut self, cmd: Cmd) -> BlockBody {
        match cmd {
            Cmd::Doctor => match glide_core::doctor(&self.ctx.repo_root) {
                Ok(r) => BlockBody::Doctor(Box::new(r)),
                Err(e) => Self::err(e),
            },

            Cmd::IndexShow => {
                let Some(db) = self.db.as_ref() else {
                    return Self::needs_init();
                };
                match db.stats() {
                    Ok(s) => BlockBody::Stats(Box::new(s)),
                    Err(e) => Self::err(e),
                }
            }

            Cmd::IndexBuild => {
                if self.ctx.safe {
                    return Self::refused("index build");
                }
                self.build_index()
            }

            Cmd::ConfigList => match self.ctx.config.to_toml() {
                Ok(t) => BlockBody::Toml(t),
                Err(e) => Self::err(e),
            },

            Cmd::ConfigGet(key) => {
                match glide_common::config_edit::lookup(&self.ctx.config, &key) {
                    Some(v) => BlockBody::Notice {
                        title: key,
                        lines: vec![v],
                    },
                    None => BlockBody::Error {
                        message: format!("no such config key: {key}"),
                    },
                }
            }

            Cmd::ConfigSet(key, value) => {
                if self.ctx.safe {
                    return Self::refused("config set");
                }
                let path = self.config_local();
                match glide_common::config_edit::set_in_file(&path, &key, &value) {
                    Err(e) => Self::err(e),
                    Ok(()) => {
                        // Re-read so later blocks see the new value.
                        match Config::load(Some(&self.ctx.repo_root)) {
                            Ok(c) => self.ctx.config = c,
                            Err(e) => return Self::err(e),
                        }
                        BlockBody::Notice {
                            title: format!("set {key} = {value}"),
                            lines: vec![path.display().to_string()],
                        }
                    }
                }
            }

            Cmd::ConfigPaths => {
                let ws = workspace_dir(&self.ctx.repo_root);
                let mut lines = vec!["1  compiled defaults".to_string()];
                if let Ok(g) = glide_common::paths::global_dir() {
                    lines.push(format!("2  {}", g.join("config.toml").display()));
                }
                lines.push(format!("3  {}", ws.join("glide.toml").display()));
                lines.push(format!("4  {}", ws.join("config.local.toml").display()));
                lines.push("5  GLIDE_* env vars".into());
                BlockBody::Notice {
                    title: "config layers, low to high".into(),
                    lines,
                }
            }

            Cmd::Models => BlockBody::Models(Box::new(glide_core::models(
                &self.ctx.config,
                self.ctx.anthropic_compiled,
            ))),

            Cmd::Focus { action, text } => self.run_focus(&action, &text),

            Cmd::ToolsStatus => BlockBody::Tools(Box::new(glide_common::tools::detect_repomix())),

            // Deliberately not run from here. A global npm install with the
            // event loop frozen and ^c unreachable is worse than handing it
            // back to the shell.
            Cmd::ToolsInstall(tool) => BlockBody::Notice {
                title: format!("installing {tool} is a shell job"),
                lines: vec![
                    format!("Quit the console and run `glide tools install {tool}`."),
                    "It shells out to npm, which wants a terminal of its own.".into(),
                ],
            },

            Cmd::FrictionLog {
                subject,
                category,
                severity,
            } => {
                if self.ctx.safe {
                    return Self::refused("friction log");
                }
                if subject.trim().is_empty() {
                    return BlockBody::Error {
                        message: "friction log needs a subject: `friction log <what happened>`"
                            .into(),
                    };
                }
                let Some(db) = self.db.as_ref() else {
                    return Self::needs_init();
                };
                match glide_core::log_event(db, &subject, &category, severity, None, None) {
                    Ok(r) => BlockBody::FrictionLogged(Box::new(r)),
                    Err(e) => Self::err(e),
                }
            }

            Cmd::FrictionDigest { since_days } => self.with_db(|db| {
                glide_core::digest(db, since_days).map(|r| BlockBody::Digest(Box::new(r)))
            }),

            Cmd::WhoOwns(query) => {
                let top = self.top;
                self.with_db(move |db| {
                    glide_core::who_owns(db, query.trim(), top, true)
                        .map(|r| BlockBody::WhoOwns(Box::new(r)))
                })
            }

            Cmd::Plan(person) => self.with_db(move |db| {
                glide_core::plan(db, person.trim(), None).map(|r| BlockBody::Plan(Box::new(r)))
            }),

            Cmd::Request(permission) => match glide_core::request(permission.trim(), None) {
                Ok(r) => BlockBody::Request(Box::new(r)),
                Err(e) => Self::err(e),
            },
        }
    }

    fn with_db<F>(&self, f: F) -> BlockBody
    where
        F: FnOnce(&GraphDb) -> anyhow::Result<BlockBody>,
    {
        let Some(db) = self.db.as_ref() else {
            return Self::needs_init();
        };
        f(db).unwrap_or_else(Self::err)
    }

    fn build_index(&mut self) -> BlockBody {
        // Drop the read handle first: the indexer wants its own writable one.
        self.db = None;
        if let Err(e) = glide_graph::init_db(&self.ctx.db_path) {
            return Self::err(e);
        }
        let mut db = match glide_graph::open(&self.ctx.db_path) {
            Ok(db) => db,
            Err(e) => return Self::err(e),
        };
        let report = glide_graph::index::run_full(&mut db, &self.ctx.repo_root, &self.ctx.config);
        self.db = Some(db);
        match report {
            Ok(r) => BlockBody::Indexed(Box::new(r)),
            Err(e) => Self::err(e),
        }
    }
}

impl Engine for GlideEngine {
    fn stats_label(&self) -> Option<String> {
        let focus = self.focus_line();
        let Some(stats) = self.db.as_ref().and_then(|d| d.stats().ok()) else {
            return focus;
        };
        let mut label = format!(
            "{} people · {} paths · {} ownership rows",
            stats.people, stats.paths, stats.ownership_rows
        );
        if stats.friction_events > 0 {
            label.push_str(&format!(" · {} friction", stats.friction_events));
        }
        if self.ctx.safe {
            label.push_str(" · safe");
        }
        if let Some(f) = focus {
            label = format!("{f}  ·  {label}");
        }
        Some(label)
    }

    fn run(&mut self, input: &str) -> BlockBody {
        // Verbs first: the router's keyword matching would swallow several of
        // them (`friction log ...` reads as a digest to it).
        if let Some(cmd) = command::parse(input) {
            return self.run_command(cmd);
        }

        let trimmed = input.trim();
        let trimmed = trimmed.strip_prefix("glide ").unwrap_or(trimmed).trim();
        let normalized = trimmed.replacen("who-owns", "who owns", 1);

        match glide_core::route_one_shot(&normalized) {
            RouteDecision::WhoOwns { query } => self.run_command(Cmd::WhoOwns(query)),
            RouteDecision::Plan { person } => self.run_command(Cmd::Plan(person)),
            RouteDecision::Request { permission } => self.run_command(Cmd::Request(permission)),
            RouteDecision::FrictionDigest => {
                self.run_command(Cmd::FrictionDigest { since_days: 7 })
            }
            RouteDecision::Unrouted { reason } => BlockBody::Unrouted {
                reason,
                suggestions: vec![
                    "who owns <path>".into(),
                    "plan for <person>".into(),
                    "i need access to <tool>".into(),
                    "doctor".into(),
                ],
            },
        }
    }
}

impl GlideEngine {
    fn vault(&self) -> Result<glide_vault::Vault, glide_vault::VaultError> {
        let v = &self.ctx.config.vault;
        glide_vault::Vault::new(&v.path, &v.daily_note_pattern).map(|vault| {
            vault.with_sections(glide_vault::Sections {
                focus: v.focus_heading.clone(),
                tasks: v.tasks_heading.clone(),
                record: v.record_heading.clone(),
                notes: v.notes_heading.clone(),
            })
        })
    }

    /// The focus strip for the top bar; `None` when no vault is configured,
    /// so the bar reads exactly as it did before the vault existed.
    fn focus_line(&self) -> Option<String> {
        let vault = self.vault().ok()?;
        let note = vault.load(glide_vault::Vault::today()).ok()?;
        Some(note.snapshot().line())
    }

    fn run_focus(&mut self, action: &str, text: &str) -> BlockBody {
        let vault = match self.vault() {
            Ok(v) => v,
            Err(e) => {
                return BlockBody::Notice {
                    title: "no vault yet".into(),
                    lines: vec![
                        e.to_string(),
                        "Set [vault] path in ~/.glide/config.toml, then try again.".into(),
                    ],
                }
            }
        };
        let mut note = match vault.load(glide_vault::Vault::today()) {
            Ok(n) => n,
            Err(e) => {
                return BlockBody::Error {
                    message: e.to_string(),
                }
            }
        };
        if action != "show" && action != "today" && self.ctx.safe {
            return BlockBody::Error {
                message: "--safe: refusing to write to the vault".into(),
            };
        }
        let headline = match action {
            "show" => None,
            "today" => {
                return BlockBody::Notice {
                    title: format!("today · {}", note.snapshot().date),
                    lines: note.raw.lines().map(String::from).collect(),
                }
            }
            "set" => Some(format!("now: {}", note.set_now(text))),
            "done" => match note.mark_done(text) {
                Ok(d) => Some(format!("done: {d}")),
                Err(e) => {
                    return BlockBody::Error {
                        message: e.to_string(),
                    }
                }
            },
            "capture" => {
                note.capture(text);
                Some(format!("captured: {}", text.trim()))
            }
            "log" => {
                note.log(text);
                Some(format!("logged: {}", text.trim()))
            }
            _ => {
                return BlockBody::Error {
                    message: format!("unknown focus action {action}"),
                }
            }
        };
        if headline.is_some() {
            if let Err(e) = note.save() {
                return BlockBody::Error {
                    message: e.to_string(),
                };
            }
        }
        let snap = note.snapshot();
        let mut lines = Vec::new();
        if let Some(h) = headline {
            lines.push(h);
        }
        lines.push(snap.line());
        for item in &snap.focus {
            let mark = if item.done { "[x]" } else { "[ ]" };
            let now = if item.now { "  ← now" } else { "" };
            lines.push(format!("  {mark} {}{now}", item.text));
        }
        BlockBody::Notice {
            title: format!("focus · {}", snap.date),
            lines,
        }
    }
}

/// The `help` block. Kept here so the key list has one source of truth.
pub fn help_body() -> BlockBody {
    BlockBody::Notice {
        title: "keys and verbs".into(),
        lines: vec![
            "enter        run what you typed".into(),
            "esc          enter block mode (esc again to leave)".into(),
            "^k           block mode, focused on the newest block".into(),
            "j / k / ↑ ↓  move focus            (block mode)".into(),
            "enter        collapse or expand    (block mode)".into(),
            "y            copy the block        (block mode)".into(),
            "r            re-run the block      (block mode)".into(),
            "^u / ^d      scroll".into(),
            "clear        drop every block".into(),
            "exit or ^c   quit".into(),
            String::new(),
            "verbs        doctor · index [build] · config [get|set|paths]".into(),
            "             models · tools · friction log|digest".into(),
            "focus        today's priorities · focus set|done|capture|log <text>".into(),
            "questions    who owns <path> · plan for <person>".into(),
            "             i need access to <tool>".into(),
        ],
    }
}

/// First block on launch, so the console is never an empty box.
pub fn welcome_body(has_graph: bool) -> BlockBody {
    let mut lines = vec![
        "Ask in plain words, or type a verb. Every answer becomes a".into(),
        "block you can focus, collapse, copy, and re-run.".into(),
        String::new(),
        "  who owns billing/charge.py".into(),
        "  i need access to pomelo".into(),
        "  doctor".into(),
    ];
    if !has_graph {
        lines.push(String::new());
        lines.push("This repo has no graph yet — run `glide init` first.".into());
    }
    lines.push(String::new());
    lines.push("Type `help` for keys and the full verb list.".into());
    BlockBody::Notice {
        title: "glide console".into(),
        lines,
    }
}
