use clap::{Args, Parser, Subcommand};
use glide_common::Stream;

#[derive(Debug, Parser)]
#[command(
    name = "glide",
    version,
    about = "glide — onboarding agent (who-owns, day-1 plans, access requests)",
    long_about = "Single Rust binary for the glide onboarding agent. Indexes the local repo into a SQLite knowledge graph, then answers ownership questions, drafts day-1 plans, and files access requests without leaving the machine."
)]
pub struct Cli {
    /// One-shot mode: route the prompt to the right verb and exit.
    #[arg(short = 'p', long, value_name = "PROMPT")]
    pub prompt: Option<String>,

    #[command(flatten)]
    pub globals: GlobalArgs,

    /// Enable verbose tracing (glide=debug). Equivalent to GLIDE_LOG=debug.
    #[arg(long, global = true)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Args, Clone)]
pub struct GlobalArgs {
    /// Output stream mode: human | ndjson | off.
    #[arg(long, value_name = "MODE", global = true)]
    pub stream: Option<Stream>,

    /// Force the final result to be structured JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable ANSI color.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Suppress non-essential output.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Strip write/network tools for this invocation (matches NCA's --safe).
    #[arg(short = 's', long, global = true)]
    pub safe: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open the block console (also what bare `glide` does on a terminal).
    Console,

    /// Bootstrap .glide/ in the current repo.
    Init(InitArgs),

    /// Manage the on-device knowledge graph.
    Index(IndexCmd),

    /// Answer ownership questions from the graph.
    #[command(name = "who-owns")]
    WhoOwns(WhoOwnsArgs),

    /// Generate a personalized day-1 plan.
    Plan(PlanArgs),

    /// Draft an access request.
    Request(RequestArgs),

    /// Log or summarize ramp-friction events.
    Friction(FrictionCmd),

    /// Read or write the layered TOML config.
    Config(ConfigCmd),

    /// Diagnostics: config, db, git, indexer health.
    Doctor,

    /// List configured LLM providers + active model.
    Models,

    /// Manage external tools (e.g. repomix).
    Tools(ToolsCmd),

    /// Keep today's priorities in view: show, set, finish, capture, log.
    Focus(FocusCmd),

    /// Serve glide's tools to a terminal agent over the Model Context Protocol.
    Mcp(McpCmd),

    /// Print the workflow context an agent needs at session start (for hooks).
    Prime(PrimeArgs),

    /// Wire glide into an agent: `claude` installs a SessionStart hook, `warp` prints the rule.
    Setup(SetupArgs),

    /// Print the paragraph to paste into any agent's instructions file.
    Onboard,

    /// Emit shell completions.
    Completion(CompletionArgs),
}

#[derive(Debug, Args)]
pub struct FocusCmd {
    #[command(subcommand)]
    pub sub: Option<FocusSub>,
}

#[derive(Debug, Subcommand)]
pub enum FocusSub {
    /// Print the one-line focus strip (default; made for tmux and prompts).
    Show,
    /// Make this the current focus; adds it to today's Focus if new.
    Set { text: Vec<String> },
    /// Check off the first Focus or Tasks bullet matching the text.
    Done { text: Vec<String> },
    /// Drop a quick note into today's Notes.
    Capture { text: Vec<String> },
    /// Append a timestamped line to today's Record.
    Log { text: Vec<String> },
    /// Print today's note as Markdown.
    Today,
}

#[derive(Debug, Args)]
pub struct PrimeArgs {
    /// Wrap the output in the SessionStart hook JSON envelope (Claude Code, Codex, Gemini CLI).
    #[arg(long)]
    pub hook_json: bool,
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    pub target: SetupTarget,
    /// Claude: write ~/.claude/settings.json instead of the project's .claude/settings.json.
    #[arg(long)]
    pub global: bool,
    /// Report whether the hook is installed, change nothing.
    #[arg(long)]
    pub check: bool,
    /// Remove the hook.
    #[arg(long)]
    pub remove: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SetupTarget {
    Claude,
    Warp,
}

#[derive(Debug, Args)]
pub struct McpCmd {
    #[command(subcommand)]
    pub sub: McpSub,
}

#[derive(Debug, Subcommand)]
pub enum McpSub {
    /// Serve over stdio. Point Claude Code, Warp, or any MCP client at `glide mcp serve`.
    Serve,
}

#[derive(Debug, Args)]
pub struct ToolsCmd {
    #[command(subcommand)]
    pub sub: ToolsSub,
}

#[derive(Debug, Subcommand)]
pub enum ToolsSub {
    /// Show which external tools glide can find.
    Status,
    /// Install a tool. Known: `repomix`.
    Install {
        #[arg(default_value = "repomix")]
        tool: String,
    },
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Overwrite an existing .glide/ if present.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct IndexCmd {
    #[command(subcommand)]
    pub sub: IndexSub,
}

#[derive(Debug, Subcommand)]
pub enum IndexSub {
    /// Run the indexer.
    Build(IndexBuildArgs),
    /// Show graph counts + last-run timestamp.
    Show(IndexShowArgs),
}

#[derive(Debug, Args)]
pub struct IndexBuildArgs {
    /// Force a full rebuild (drop + reinsert).
    #[arg(long)]
    pub full: bool,
    /// (v1.1) Index only commits since this git ref. Ignored in v1.
    #[arg(long, value_name = "REF")]
    pub since: Option<String>,
}

#[derive(Debug, Args)]
pub struct IndexShowArgs {
    /// Optional path to filter ownership rows by.
    #[arg(long)]
    pub path: Option<String>,
}

#[derive(Debug, Args)]
pub struct WhoOwnsArgs {
    /// File path or glob to resolve.
    pub query: String,
    /// Max owners to return.
    #[arg(long, default_value_t = 5)]
    pub top: usize,
    /// Include the evidence column in human output.
    #[arg(long)]
    pub why: bool,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Person handle or display name.
    pub person: String,
    /// Optional role.
    #[arg(long)]
    pub role: Option<String>,
    /// Write to a file instead of stdout.
    #[arg(long)]
    pub out: Option<std::path::PathBuf>,
}

#[derive(Debug, Args)]
pub struct RequestArgs {
    /// Permission name (e.g. "staging-db", "admin:billing").
    pub permission: String,
    /// Person the request is for.
    #[arg(long = "for")]
    pub for_person: Option<String>,
    /// Print but never attempt to send (default in v1).
    #[arg(long)]
    pub draft: bool,
}

#[derive(Debug, Args)]
pub struct FrictionCmd {
    #[command(subcommand)]
    pub sub: FrictionSub,
}

#[derive(Debug, Subcommand)]
pub enum FrictionSub {
    /// Append a friction event to the local graph.
    Log(FrictionLogArgs),
    /// Roll events into a digest.
    Digest(FrictionDigestArgs),
}

#[derive(Debug, Args)]
pub struct FrictionLogArgs {
    /// Short subject line: "needs access to staging-db".
    pub subject: String,
    /// Category: access | ownership-unknown | tooling | doc-gap.
    #[arg(long, default_value = "access")]
    pub category: String,
    #[arg(long, default_value_t = 3)]
    pub severity: u8,
    #[arg(long)]
    pub person: Option<String>,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct FrictionDigestArgs {
    /// Window size in days.
    #[arg(long, default_value_t = 7)]
    pub since_days: i64,
}

#[derive(Debug, Args)]
pub struct ConfigCmd {
    #[command(subcommand)]
    pub sub: ConfigSub,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSub {
    /// Print effective config as TOML.
    List,
    /// Read one key (dotted path).
    Get { key: String },
    /// Set one key in the workspace local config.
    Set { key: String, value: String },
    /// Print the path of each config layer.
    Paths,
}

#[derive(Debug, Args)]
pub struct CompletionArgs {
    /// Shell: bash | zsh | fish | powershell | elvish.
    pub shell: clap_complete::Shell,
}
