//! `glide mcp serve --http` and `glide mcp up|down|status`: the vault server
//! over streamable HTTP, behind OAuth, reachable from the internet through
//! Tailscale Funnel. The server binds to loopback only; Funnel is the one
//! way in, and every `/mcp` request needs a token from `mcp_auth`.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use glide_common::config::McpHttpConfig;
use glide_common::Config;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::cli::GlobalArgs;
use crate::commands::mcp::{notes_for, VaultServer};
use crate::commands::mcp_auth::{self, Auth, Caller, GithubApp};
use crate::commands::open_vault;

const KEYCHAIN_SERVICE: &str = "glide-mcp-github";

fn glide_dir() -> Result<PathBuf> {
    Ok(directories::BaseDirs::new()
        .context("no home directory")?
        .home_dir()
        .join(".glide"))
}

fn access_log() -> Result<PathBuf> {
    Ok(glide_dir()?.join("mcp-access.log"))
}

fn load_config() -> Result<McpHttpConfig> {
    let cwd = std::env::current_dir()?;
    Ok(
        Config::load(glide_common::paths::find_repo_root(&cwd).as_deref())?
            .mcp
            .http,
    )
}

fn port_of(addr: &str) -> Result<u16> {
    addr.rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .with_context(|| format!("[mcp.http] addr has no port: {addr}"))
}

/// `https://<machine>.<tailnet>.ts.net` for this machine, from `tailscale status`.
fn tailscale_url() -> Result<String> {
    let out = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .context("tailscale is not installed")?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).context(
        "could not read `tailscale status --json`; is Tailscale running? Try `tailscale up`.",
    )?;
    if v["BackendState"].as_str() != Some("Running") {
        bail!(
            "Tailscale is {}; run `tailscale up` first",
            v["BackendState"].as_str().unwrap_or("not running")
        );
    }
    let dns = v["Self"]["DNSName"]
        .as_str()
        .unwrap_or("")
        .trim_end_matches('.');
    if dns.is_empty() {
        bail!("Tailscale has no DNS name for this machine; turn on MagicDNS and HTTPS in the admin console");
    }
    Ok(format!("https://{dns}"))
}

fn github_secret(client_id: &str) -> Result<String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            client_id,
            "-w",
        ])
        .output()
        .context("could not run the macOS `security` tool")?;
    if !out.status.success() {
        bail!(
            "the GitHub OAuth App secret is not in the Keychain. Add it once with:\n  \
             security add-generic-password -s {KEYCHAIN_SERVICE} -a {client_id} -w\n\
             (it prompts for the secret, so it never lands in your shell history)"
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// One line per `/mcp` request: time, GitHub login, and the tool when it is a tool call.
async fn log_access(State(log): State<Arc<Option<PathBuf>>>, req: Request, next: Next) -> Response {
    let login = req
        .extensions()
        .get::<Caller>()
        .map(|c| c.0.clone())
        .unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 4 << 20)
        .await
        .unwrap_or_default();
    let what = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .map(
            |v| match (v["method"].as_str(), v["params"]["name"].as_str()) {
                (Some("tools/call"), Some(tool)) => format!("tool {tool}"),
                (Some(m), _) => m.to_string(),
                _ => String::new(),
            },
        )
        .unwrap_or_default();
    if !what.is_empty() {
        if let Some(path) = log.as_ref() {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(
                    f,
                    "{}\t{login}\t{what}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
    }
    next.run(Request::from_parts(parts, axum::body::Body::from(bytes)))
        .await
}

/// The whole HTTP surface: OAuth metadata and endpoints, and `/mcp` behind a token.
pub fn router(
    server: VaultServer,
    auth: Auth,
    public_host: &str,
    access_log: Option<PathBuf>,
) -> Router {
    let mut hosts: Vec<String> = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
    hosts.push(public_host.to_string());
    let config = StreamableHttpServerConfig::default().with_allowed_hosts(hosts);
    let factory = Arc::new(server);
    let mcp = StreamableHttpService::new(
        move || Ok((*factory).clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let gated = Router::new()
        .nest_service("/mcp", mcp)
        .layer(middleware::from_fn_with_state(
            Arc::new(access_log),
            log_access,
        ))
        .layer(middleware::from_fn_with_state(
            auth.clone(),
            mcp_auth::require_token,
        ));
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(mcp_auth::protected_resource),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(mcp_auth::protected_resource),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(mcp_auth::authorization_server),
        )
        .route("/oauth/register", post(mcp_auth::register))
        .route("/oauth/authorize", get(mcp_auth::authorize))
        .route("/oauth/callback", get(mcp_auth::callback))
        .route("/oauth/token", post(mcp_auth::token))
        .route(
            "/",
            get(|| async { "glide vault MCP server. Connect your AI app to /mcp." }),
        )
        .with_state(auth)
        .merge(gated)
}

pub fn serve(globals: &GlobalArgs, addr: Option<&str>, tunnel: bool) -> Result<()> {
    let cfg = load_config()?;
    let addr = addr.unwrap_or(&cfg.addr).to_string();
    let port = port_of(&addr)?;
    if !addr.starts_with("127.0.0.1:")
        && !addr.starts_with("localhost:")
        && !addr.starts_with("[::1]:")
    {
        bail!("refusing to bind {addr}: glide only listens on loopback; Funnel is the way in from outside");
    }
    if cfg.allow.is_empty() {
        bail!("[mcp.http] allow is empty, so nobody could sign in. Add your GitHub login, e.g.\n  [mcp.http]\n  allow = [\"your-github-login\"]");
    }
    if cfg.github_client_id.is_empty() {
        bail!("[mcp.http] github_client_id is empty. Create a GitHub OAuth App (github.com/settings/developers) and put its client id there");
    }
    let public_url = if !cfg.public_url.is_empty() {
        cfg.public_url.trim_end_matches('/').to_string()
    } else if tunnel {
        tailscale_url()?
    } else {
        format!("http://{addr}")
    };
    let secret = github_secret(&cfg.github_client_id)?;
    let vault = open_vault()?;
    let notes = notes_for(&vault)?;
    let server = VaultServer::new(vault, notes, globals.safe, true);
    let auth = Auth::open(
        &glide_dir()?.join("mcp-auth.db"),
        &public_url,
        &cfg.allow,
        Box::new(GithubApp {
            client_id: cfg.github_client_id.clone(),
            client_secret: secret,
        }),
    )?;
    let public_host = url::Url::parse(&public_url)?
        .host_str()
        .unwrap_or("localhost")
        .to_string();
    let app = router(server, auth, &public_host, Some(access_log()?));

    if tunnel {
        let st = Command::new("tailscale")
            .args(["funnel", "--bg", &port.to_string()])
            .status()?;
        if !st.success() {
            bail!("`tailscale funnel --bg {port}` failed. Funnel has to be allowed for this machine in the Tailscale admin console (Access controls → funnel node attribute)");
        }
    }

    println!("glide vault MCP");
    println!("  connect to   {public_url}/mcp");
    println!("  sign-in      GitHub as {}", cfg.allow.join(", "));
    println!(
        "  hidden       {}",
        if cfg.exclude.is_empty() {
            "nothing".into()
        } else {
            cfg.exclude.join(", ")
        }
    );
    println!("  GitHub app callback must be {public_url}/oauth/callback");
    if tunnel {
        println!();
        println!("  Claude Code  claude mcp add --transport http glide {public_url}/mcp");
        println!("  claude.ai    Settings → Connectors → Add custom connector → {public_url}/mcp");
        println!("  ChatGPT      Settings → Apps → Advanced → Developer mode → Create → {public_url}/mcp");
        println!("  Cursor       ~/.cursor/mcp.json: {{\"mcpServers\":{{\"glide\":{{\"url\":\"{public_url}/mcp\"}}}}}}");
        println!("  Codex        codex mcp add glide --url {public_url}/mcp");
        println!();
        println!("  Ctrl-C stops the server; `glide mcp down` turns the Funnel off.");
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("could not bind {addr}"))?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
        Ok::<(), anyhow::Error>(())
    })?;
    if tunnel {
        let _ = Command::new("tailscale").args(["funnel", "reset"]).status();
        println!("Funnel off.");
    }
    Ok(())
}

pub fn down() -> Result<()> {
    let st = Command::new("tailscale")
        .args(["funnel", "reset"])
        .status()
        .context("tailscale is not installed")?;
    if !st.success() {
        bail!("`tailscale funnel reset` failed");
    }
    println!("Funnel off. The vault is no longer reachable from the internet.");
    Ok(())
}

/// "on, forwarding to 127.0.0.1:8787" or "off", from `tailscale funnel status --json`.
fn funnel_line(addr: &str) -> String {
    let Ok(out) = Command::new("tailscale")
        .args(["funnel", "status", "--json"])
        .output()
    else {
        return "unknown (tailscale is not installed)".into();
    };
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let on = v["AllowFunnel"]
        .as_object()
        .is_some_and(|m| m.values().any(|b| b.as_bool() == Some(true)));
    if !on {
        return "off".into();
    }
    let proxies: Vec<String> = v["Web"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter_map(|site| site["Handlers"]["/"]["Proxy"].as_str())
        .map(|p| p.trim_start_matches("http://").to_string())
        .collect();
    if proxies.iter().any(|p| p == addr) {
        format!("on, forwarding to {addr}")
    } else {
        format!(
            "on, but forwarding to {} instead of {addr}",
            proxies.join(", ")
        )
    }
}

pub fn status() -> Result<()> {
    let cfg = load_config()?;
    let funnel = funnel_line(&cfg.addr);
    println!("server       {}", cfg.addr);
    println!(
        "public url   {}",
        if cfg.public_url.is_empty() {
            tailscale_url().unwrap_or_else(|e| e.to_string())
        } else {
            cfg.public_url.clone()
        }
    );
    println!(
        "allow        {}",
        if cfg.allow.is_empty() {
            "nobody".into()
        } else {
            cfg.allow.join(", ")
        }
    );
    println!(
        "hidden       {}",
        if cfg.exclude.is_empty() {
            "nothing".into()
        } else {
            cfg.exclude.join(", ")
        }
    );
    println!("funnel       {funnel}");
    let log = std::fs::read_to_string(access_log()?).unwrap_or_default();
    let recent: Vec<&str> = log.lines().rev().take(10).collect();
    println!(
        "recent       {}",
        if recent.is_empty() { "none" } else { "" }
    );
    for line in recent.iter().rev() {
        println!("  {line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request as HttpRequest, StatusCode};
    use tower::ServiceExt;

    struct NoGithub;
    impl mcp_auth::GithubIdentity for NoGithub {
        fn authorize_url(&self, s: &str, _: &str) -> String {
            format!("https://github.test/?state={s}")
        }
        fn login_for_code(&self, _: &str, _: &str) -> anyhow::Result<String> {
            anyhow::bail!("unused")
        }
    }

    fn app() -> (tempfile::TempDir, Router) {
        let d = tempfile::tempdir().unwrap();
        let vault_dir = d.path().join("vault");
        std::fs::create_dir_all(vault_dir.join("Personal")).unwrap();
        std::fs::write(vault_dir.join("Personal/secret.md"), "private").unwrap();
        let vault =
            glide_vault::Vault::new(vault_dir.to_str().unwrap(), "Daily Notes/{date}.md").unwrap();
        let notes =
            crate::commands::mcp_notes::Notes::new(&vault.root, &["Personal".into()]).unwrap();
        let server = VaultServer::new(vault, notes, false, true);
        let auth = Auth::open(
            &d.path().join("auth.db"),
            "https://vault.test",
            &["me".into()],
            Box::new(NoGithub),
        )
        .unwrap();
        (d, router(server, auth, "vault.test", None))
    }

    /// Same app, plus a vault with a public note and a live token for `me`.
    fn signed_in_app() -> (tempfile::TempDir, Router, String) {
        let d = tempfile::tempdir().unwrap();
        let vault_dir = d.path().join("vault");
        std::fs::create_dir_all(vault_dir.join("Personal")).unwrap();
        std::fs::create_dir_all(vault_dir.join("Projects")).unwrap();
        std::fs::write(vault_dir.join("Personal/secret.md"), "private pineapple").unwrap();
        std::fs::write(vault_dir.join("Projects/Glide.md"), "public pineapple plan").unwrap();
        let vault =
            glide_vault::Vault::new(vault_dir.to_str().unwrap(), "Daily Notes/{date}.md").unwrap();
        let notes =
            crate::commands::mcp_notes::Notes::new(&vault.root, &["Personal".into()]).unwrap();
        let server = VaultServer::new(vault, notes, false, true);
        let auth = Auth::open(
            &d.path().join("auth.db"),
            "https://vault.test",
            &["me".into()],
            Box::new(NoGithub),
        )
        .unwrap();
        let token = auth.issue_tokens("c", "me").unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_string();
        (d, router(server, auth, "vault.test", None), token)
    }

    /// POST one JSON-RPC message to /mcp and return (session id, the JSON result from the SSE or JSON body).
    async fn rpc(
        app: &Router,
        token: &str,
        session: Option<&str>,
        body: serde_json::Value,
    ) -> (Option<String>, serde_json::Value) {
        let mut req = HttpRequest::post("/mcp")
            .header(header::HOST, "vault.test")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");
        if let Some(s) = session {
            req = req.header("mcp-session-id", s);
        }
        let res = app
            .clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        assert!(res.status().is_success(), "status {}", res.status());
        let sid = res
            .headers()
            .get("mcp-session-id")
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let json = text
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .find_map(|l| serde_json::from_str(l).ok())
            .or_else(|| serde_json::from_str(&text).ok())
            .unwrap_or(serde_json::Value::Null);
        (sid, json)
    }

    #[tokio::test]
    async fn signed_in_client_can_search_and_read_but_not_see_hidden_notes_or_log() {
        let (_d, app, token) = signed_in_app();
        let (sid, init) = rpc(&app, &token, None, serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}
        })).await;
        assert_eq!(init["result"]["serverInfo"]["name"], "glide");
        let sid = sid.expect("session id");
        let _ = rpc(
            &app,
            &token,
            Some(&sid),
            serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )
        .await;

        let (_, tools) = rpc(
            &app,
            &token,
            Some(&sid),
            serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        )
        .await;
        let names: Vec<String> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        for want in [
            "search_notes",
            "read_note",
            "list_notes",
            "backlinks",
            "today",
            "capture",
        ] {
            assert!(
                names.contains(&want.to_string()),
                "missing {want} in {names:?}"
            );
        }
        assert!(
            !names.contains(&"log".to_string()),
            "log must not be offered remotely"
        );

        let (_, found) = rpc(&app, &token, Some(&sid), serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "search_notes", "arguments": {"query": "pineapple"}}
        })).await;
        let found = found["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(found.contains("Projects/Glide.md"));
        assert!(
            !found.contains("Personal"),
            "hidden folder leaked through search: {found}"
        );

        let (_, hidden) = rpc(&app, &token, Some(&sid), serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "read_note", "arguments": {"path": "Personal/secret"}}
        })).await;
        assert!(
            !hidden.to_string().contains("private pineapple"),
            "hidden note was readable: {hidden}"
        );
    }

    #[tokio::test]
    async fn mcp_without_a_token_is_401_with_where_to_sign_in() {
        let (_d, app) = app();
        let req = HttpRequest::post("/mcp")
            .header(header::HOST, "vault.test")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_note","arguments":{"path":"Personal/secret"}}}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let www = res.headers()[header::WWW_AUTHENTICATE]
            .to_str()
            .unwrap()
            .to_string();
        assert!(www.contains("https://vault.test/.well-known/oauth-protected-resource"));
        let body = axum::body::to_bytes(res.into_body(), 1 << 16)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("private"));
    }

    #[tokio::test]
    async fn metadata_is_public_and_points_at_this_server() {
        let (_d, app) = app();
        let res = app
            .oneshot(
                HttpRequest::get("/.well-known/oauth-authorization-server")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let v: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(res.into_body(), 1 << 16)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(v["token_endpoint"], "https://vault.test/oauth/token");
        assert_eq!(v["code_challenge_methods_supported"][0], "S256");
    }

    #[test]
    fn port_is_read_from_the_addr() {
        assert_eq!(port_of("127.0.0.1:8787").unwrap(), 8787);
        assert!(port_of("nope").is_err());
    }
}
