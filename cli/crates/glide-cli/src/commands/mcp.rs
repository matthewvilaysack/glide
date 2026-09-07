//! `glide mcp serve`: the same vault verbs as `glide focus`, offered to
//! whatever agent is running in the terminal. stdio only, no daemon, no
//! network. Every tool reads the note fresh, so the agent and the human can
//! edit the same file without either one holding a lock.

use anyhow::Result;
use glide_vault::Vault;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, ErrorData as McpError, Implementation, ServerCapabilities,
    ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};

use crate::cli::{GlobalArgs, McpCmd, McpSub};
use crate::commands::open_vault;

pub fn run(globals: &GlobalArgs, cmd: McpCmd) -> Result<()> {
    match cmd.sub {
        McpSub::Serve => serve(globals),
    }
}

fn serve(globals: &GlobalArgs) -> Result<()> {
    let vault = open_vault()?;
    let server = VaultServer::new(vault, globals.safe);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let service = server.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok::<(), anyhow::Error>(())
    })
}

#[derive(Clone)]
struct VaultServer {
    vault: Vault,
    safe: bool,
    #[allow(dead_code)] // read by the tool_handler macro
    tool_router: ToolRouter<VaultServer>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct TextArg {
    /// The text. For focus_set and focus_done a case-insensitive substring of an existing bullet is enough.
    text: String,
}

fn text(s: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(s.into())])
}

fn json<T: serde::Serialize>(v: &T) -> Result<CallToolResult, McpError> {
    serde_json::to_string_pretty(v)
        .map(text)
        .map_err(|e| McpError::internal_error(e.to_string(), None))
}

fn fail(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

#[tool_router]
impl VaultServer {
    fn new(vault: Vault, safe: bool) -> Self {
        Self {
            vault,
            safe,
            tool_router: Self::tool_router(),
        }
    }

    fn load(&self) -> Result<glide_vault::Daily, McpError> {
        self.vault.load(Vault::today()).map_err(fail)
    }

    fn writable(&self) -> Result<(), McpError> {
        if self.safe {
            return Err(McpError::invalid_request(
                "glide was started with --safe; vault writes are off",
                None,
            ));
        }
        Ok(())
    }

    #[tool(
        description = "Today's priorities at a glance: current focus (the bullet tagged #now), the Focus list with done flags, open and done task counts, and how many Record and Notes entries exist. Call this before starting work and whenever the person asks what they should be doing."
    )]
    fn today(&self) -> Result<CallToolResult, McpError> {
        json(&self.load()?.snapshot())
    }

    #[tool(
        description = "The full text of today's daily note as Markdown, for when the snapshot is not enough."
    )]
    fn read_today(&self) -> Result<CallToolResult, McpError> {
        Ok(text(self.load()?.raw))
    }

    #[tool(
        description = "Make this the current focus. Matches an existing Focus bullet by substring, otherwise adds a new one. Use when the person says what they are working on now."
    )]
    fn focus_set(
        &self,
        Parameters(TextArg { text: t }): Parameters<TextArg>,
    ) -> Result<CallToolResult, McpError> {
        self.writable()?;
        let mut n = self.load()?;
        let chosen = n.set_now(&t);
        n.save().map_err(fail)?;
        Ok(text(format!("now: {chosen}\n{}", n.snapshot().line())))
    }

    #[tool(
        description = "Check off the first Focus or Tasks bullet matching the text. Use when something the person planned is finished."
    )]
    fn focus_done(
        &self,
        Parameters(TextArg { text: t }): Parameters<TextArg>,
    ) -> Result<CallToolResult, McpError> {
        self.writable()?;
        let mut n = self.load()?;
        let done = n.mark_done(&t).map_err(fail)?;
        n.save().map_err(fail)?;
        Ok(text(format!("done: {done}\n{}", n.snapshot().line())))
    }

    #[tool(
        description = "Drop a quick note into today's Notes section: an idea, a thing to remember, a question for later. One line."
    )]
    fn capture(
        &self,
        Parameters(TextArg { text: t }): Parameters<TextArg>,
    ) -> Result<CallToolResult, McpError> {
        self.writable()?;
        let mut n = self.load()?;
        n.capture(&t);
        n.save().map_err(fail)?;
        Ok(text(format!("captured: {}", t.trim())))
    }

    #[tool(
        description = "Append a timestamped line to today's Record: what actually happened. Call this at the end of every task you complete for the person, in one or two sentences, without being asked."
    )]
    fn log(
        &self,
        Parameters(TextArg { text: t }): Parameters<TextArg>,
    ) -> Result<CallToolResult, McpError> {
        self.writable()?;
        let mut n = self.load()?;
        n.log(&t);
        n.save().map_err(fail)?;
        Ok(text(format!("logged: {}", t.trim())))
    }
}

fn server_identity() -> Implementation {
    let mut id = Implementation::from_build_env();
    id.name = "glide".into();
    id.version = env!("CARGO_PKG_VERSION").into();
    id
}

#[tool_handler]
impl ServerHandler for VaultServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_identity())
            .with_instructions(
                "glide keeps this person's daily priorities in a Markdown note they own. \
                 Call `today` at the start of a session and mention the current focus in one line. \
                 When they say what they are working on, call `focus_set`. When something finishes, `focus_done`. \
                 After every task you complete, call `log` with one or two sentences. \
                 Use `capture` for anything they say to remember. Never rewrite the note yourself; these tools are the only writers."
                    .to_string(),
            )
    }
}
