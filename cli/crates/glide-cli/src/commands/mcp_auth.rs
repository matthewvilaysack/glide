//! OAuth 2.1 for the remote MCP server. Glide is the authorization server and
//! hands identity to GitHub: a client registers itself, the person signs in
//! with GitHub, and only logins on `[mcp.http] allow` get a token. Codes and
//! tokens are random and stored only as SHA-256 hashes.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Form, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

const CODE_TTL: i64 = 5 * 60;
const ACCESS_TTL: i64 = 60 * 60;
const REFRESH_TTL: i64 = 30 * 24 * 60 * 60;
const PENDING_TTL: Duration = Duration::from_secs(10 * 60);
/// Requests per minute across the unauthenticated write endpoints.
const RATE_PER_MIN: u32 = 30;

/// Who the authenticated caller is, attached to `/mcp` requests.
#[derive(Clone, Debug)]
pub struct Caller(pub String);

#[derive(Clone)]
pub struct Auth {
    inner: Arc<Inner>,
}

struct Inner {
    public_url: String,
    allow: Vec<String>,
    github: Box<dyn GithubIdentity>,
    db: Mutex<Connection>,
    pending: Mutex<HashMap<String, Pending>>,
    rate: Mutex<(Instant, u32)>,
}

struct Pending {
    client_id: String,
    redirect_uri: String,
    challenge: String,
    client_state: Option<String>,
    at: Instant,
}

/// The one thing Glide needs from GitHub: which login finished sign-in.
/// A trait so the tests can stand in for GitHub.
pub trait GithubIdentity: Send + Sync {
    fn authorize_url(&self, state: &str, callback: &str) -> String;
    fn login_for_code(&self, code: &str, callback: &str) -> anyhow::Result<String>;
}

pub struct GithubApp {
    pub client_id: String,
    pub client_secret: String,
}

impl GithubIdentity for GithubApp {
    fn authorize_url(&self, state: &str, callback: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&allow_signup=false",
            enc(&self.client_id),
            enc(callback),
            enc(state)
        )
    }

    fn login_for_code(&self, code: &str, callback: &str) -> anyhow::Result<String> {
        let http = reqwest::blocking::Client::builder()
            .user_agent("glide-mcp")
            .timeout(Duration::from_secs(15))
            .build()?;
        let tok: serde_json::Value = http
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", callback),
            ])
            .send()?
            .error_for_status()?
            .json()?;
        let access = tok["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("GitHub refused the sign-in code: {tok}"))?;
        let user: serde_json::Value = http
            .get("https://api.github.com/user")
            .bearer_auth(access)
            .send()?
            .error_for_status()?
            .json()?;
        user["login"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("GitHub returned no login"))
    }
}

fn enc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn random_token() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("system randomness");
    URL_SAFE_NO_PAD.encode(b)
}

fn hash(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn oauth_error(status: StatusCode, code: &str, desc: &str) -> Response {
    (
        status,
        Json(json!({ "error": code, "error_description": desc })),
    )
        .into_response()
}

impl Auth {
    pub fn open(
        db_path: &Path,
        public_url: &str,
        allow: &[String],
        github: Box<dyn GithubIdentity>,
    ) -> anyhow::Result<Self> {
        if let Some(dir) = db_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let db = Connection::open(db_path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS clients (id TEXT PRIMARY KEY, name TEXT, redirect_uris TEXT NOT NULL, created INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS codes (hash TEXT PRIMARY KEY, client_id TEXT NOT NULL, redirect_uri TEXT NOT NULL, challenge TEXT NOT NULL, login TEXT NOT NULL, expires INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS tokens (hash TEXT PRIMARY KEY, kind TEXT NOT NULL, client_id TEXT NOT NULL, login TEXT NOT NULL, expires INTEGER NOT NULL);",
        )?;
        Ok(Self {
            inner: Arc::new(Inner {
                public_url: public_url.trim_end_matches('/').to_string(),
                allow: allow.iter().map(|a| a.to_lowercase()).collect(),
                github,
                db: Mutex::new(db),
                pending: Mutex::new(HashMap::new()),
                rate: Mutex::new((Instant::now(), 0)),
            }),
        })
    }

    fn base(&self) -> &str {
        &self.inner.public_url
    }

    fn callback(&self) -> String {
        format!("{}/oauth/callback", self.base())
    }

    pub fn resource_metadata_url(&self) -> String {
        format!("{}/.well-known/oauth-protected-resource", self.base())
    }

    fn allowed(&self, login: &str) -> bool {
        self.inner.allow.iter().any(|a| a == &login.to_lowercase())
    }

    fn rate_ok(&self) -> bool {
        let mut r = self.inner.rate.lock().unwrap();
        if r.0.elapsed() > Duration::from_secs(60) {
            *r = (Instant::now(), 0);
        }
        r.1 += 1;
        r.1 <= RATE_PER_MIN
    }

    pub(crate) fn issue_tokens(
        &self,
        client_id: &str,
        login: &str,
    ) -> rusqlite::Result<serde_json::Value> {
        let access = random_token();
        let refresh = random_token();
        let db = self.inner.db.lock().unwrap();
        db.execute(
            "INSERT INTO tokens (hash, kind, client_id, login, expires) VALUES (?1, 'access', ?2, ?3, ?4)",
            params![hash(&access), client_id, login, now() + ACCESS_TTL],
        )?;
        db.execute(
            "INSERT INTO tokens (hash, kind, client_id, login, expires) VALUES (?1, 'refresh', ?2, ?3, ?4)",
            params![hash(&refresh), client_id, login, now() + REFRESH_TTL],
        )?;
        db.execute("DELETE FROM tokens WHERE expires < ?1", params![now()])?;
        Ok(json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": ACCESS_TTL,
            "refresh_token": refresh,
        }))
    }

    /// The login behind a bearer token, if it is a live access token for someone still allowed.
    pub fn check_bearer(&self, token: &str) -> Option<String> {
        let db = self.inner.db.lock().unwrap();
        let login: Option<String> = db
            .query_row(
                "SELECT login FROM tokens WHERE hash = ?1 AND kind = 'access' AND expires > ?2",
                params![hash(token), now()],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        login.filter(|l| self.allowed(l))
    }
}

pub async fn protected_resource(State(a): State<Auth>) -> Json<serde_json::Value> {
    Json(json!({
        "resource": format!("{}/mcp", a.base()),
        "authorization_servers": [a.base()],
        "bearer_methods_supported": ["header"],
    }))
}

pub async fn authorization_server(State(a): State<Auth>) -> Json<serde_json::Value> {
    let b = a.base();
    Json(json!({
        "issuer": b,
        "authorization_endpoint": format!("{b}/oauth/authorize"),
        "token_endpoint": format!("{b}/oauth/token"),
        "registration_endpoint": format!("{b}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

#[derive(Deserialize)]
pub struct RegisterReq {
    redirect_uris: Vec<String>,
    client_name: Option<String>,
}

fn acceptable_redirect(uri: &str) -> bool {
    match url::Url::parse(uri) {
        Ok(u) => {
            u.scheme() == "https"
                || (u.scheme() == "http"
                    && matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")))
                // Native apps (Cursor, VS Code) use their own URL schemes.
                || !matches!(u.scheme(), "http" | "https" | "javascript" | "data" | "file")
        }
        Err(_) => false,
    }
}

pub async fn register(State(a): State<Auth>, Json(req): Json<RegisterReq>) -> Response {
    if !a.rate_ok() {
        return oauth_error(
            StatusCode::TOO_MANY_REQUESTS,
            "slow_down",
            "too many requests",
        );
    }
    if req.redirect_uris.is_empty() || !req.redirect_uris.iter().all(|u| acceptable_redirect(u)) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect URIs must be https, localhost, or an app scheme",
        );
    }
    let id = random_token();
    let uris = serde_json::to_string(&req.redirect_uris).unwrap();
    let res = a.inner.db.lock().unwrap().execute(
        "INSERT INTO clients (id, name, redirect_uris, created) VALUES (?1, ?2, ?3, ?4)",
        params![id, req.client_name, uris, now()],
    );
    if let Err(e) = res {
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            &e.to_string(),
        );
    }
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": id,
            "client_name": req.client_name,
            "redirect_uris": req.redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
        })),
    )
        .into_response()
}

fn client_redirects(a: &Auth, client_id: &str) -> Option<Vec<String>> {
    let db = a.inner.db.lock().unwrap();
    let raw: Option<String> = db
        .query_row(
            "SELECT redirect_uris FROM clients WHERE id = ?1",
            params![client_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    raw.and_then(|r| serde_json::from_str(&r).ok())
}

#[derive(Deserialize)]
pub struct AuthorizeReq {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
}

pub async fn authorize(State(a): State<Auth>, Query(q): Query<AuthorizeReq>) -> Response {
    // Redirect URI problems are shown here, never redirected to, per RFC 6749 4.1.2.1.
    let Some(uris) = client_redirects(&a, &q.client_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Html("Unknown client. Remove the connector and add it again."),
        )
            .into_response();
    };
    if !uris.contains(&q.redirect_uri) {
        return (
            StatusCode::BAD_REQUEST,
            Html("That redirect address is not registered for this client."),
        )
            .into_response();
    }
    let challenge = match (q.code_challenge, q.code_challenge_method.as_deref()) {
        (Some(c), Some("S256")) if !c.is_empty() => c,
        _ => return (StatusCode::BAD_REQUEST, Html("PKCE with S256 is required.")).into_response(),
    };
    if q.response_type != "code" {
        return (
            StatusCode::BAD_REQUEST,
            Html("Only response_type=code is supported."),
        )
            .into_response();
    }
    let gh_state = random_token();
    {
        let mut p = a.inner.pending.lock().unwrap();
        p.retain(|_, v| v.at.elapsed() < PENDING_TTL);
        p.insert(
            gh_state.clone(),
            Pending {
                client_id: q.client_id,
                redirect_uri: q.redirect_uri,
                challenge,
                client_state: q.state,
                at: Instant::now(),
            },
        );
    }
    Redirect::to(&a.inner.github.authorize_url(&gh_state, &a.callback())).into_response()
}

#[derive(Deserialize)]
pub struct CallbackReq {
    code: Option<String>,
    state: String,
    error: Option<String>,
}

fn page(status: StatusCode, msg: &str) -> Response {
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'>\
         <title>glide</title><body style='font:16px system-ui;max-width:32rem;margin:4rem auto;padding:0 1rem'>\
         <p>{msg}</p></body>"
    );
    (status, Html(body)).into_response()
}

pub async fn callback(State(a): State<Auth>, Query(q): Query<CallbackReq>) -> Response {
    let pending = a.inner.pending.lock().unwrap().remove(&q.state);
    let Some(p) = pending.filter(|p| p.at.elapsed() < PENDING_TTL) else {
        return page(
            StatusCode::BAD_REQUEST,
            "This sign-in link expired. Start again from your AI app.",
        );
    };
    if let Some(e) = q.error {
        return page(
            StatusCode::BAD_REQUEST,
            &format!("GitHub sign-in was cancelled ({e})."),
        );
    }
    let Some(gh_code) = q.code else {
        return page(StatusCode::BAD_REQUEST, "GitHub sent no sign-in code.");
    };
    let a2 = a.clone();
    let callback = a.callback();
    let login = match tokio::task::spawn_blocking(move || {
        a2.inner.github.login_for_code(&gh_code, &callback)
    })
    .await
    {
        Ok(Ok(l)) => l,
        Ok(Err(e)) => {
            return page(
                StatusCode::BAD_GATEWAY,
                &format!("GitHub sign-in failed: {e}"),
            )
        }
        Err(e) => return page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    if !a.allowed(&login) {
        tracing::warn!(login, "sign-in refused: not on [mcp.http] allow");
        return page(
            StatusCode::FORBIDDEN,
            &format!("@{login} isn't allowed to open this vault."),
        );
    }
    let code = random_token();
    let res = a.inner.db.lock().unwrap().execute(
        "INSERT INTO codes (hash, client_id, redirect_uri, challenge, login, expires) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![hash(&code), p.client_id, p.redirect_uri, p.challenge, login, now() + CODE_TTL],
    );
    if let Err(e) = res {
        return page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    let mut url = match url::Url::parse(&p.redirect_uri) {
        Ok(u) => u,
        Err(e) => return page(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    url.query_pairs_mut().append_pair("code", &code);
    if let Some(s) = p.client_state {
        url.query_pairs_mut().append_pair("state", &s);
    }
    Redirect::to(url.as_str()).into_response()
}

#[derive(Deserialize)]
pub struct TokenReq {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

pub async fn token(State(a): State<Auth>, Form(t): Form<TokenReq>) -> Response {
    if !a.rate_ok() {
        return oauth_error(
            StatusCode::TOO_MANY_REQUESTS,
            "slow_down",
            "too many requests",
        );
    }
    let bad = |d: &str| oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", d);
    let client_id = t.client_id.unwrap_or_default();
    match t.grant_type.as_str() {
        "authorization_code" => {
            let (Some(code), Some(verifier), Some(redirect)) =
                (t.code, t.code_verifier, t.redirect_uri)
            else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "code, code_verifier and redirect_uri are required",
                );
            };
            // Single use: the row is deleted whether or not the rest checks out.
            let row: Option<(String, String, String, String, i64)> = {
                let db = a.inner.db.lock().unwrap();
                let row = db
                    .query_row(
                        "SELECT client_id, redirect_uri, challenge, login, expires FROM codes WHERE hash = ?1",
                        params![hash(&code)],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                    )
                    .optional()
                    .ok()
                    .flatten();
                let _ = db.execute("DELETE FROM codes WHERE hash = ?1", params![hash(&code)]);
                row
            };
            let Some((cid, ruri, challenge, login, expires)) = row else {
                return bad("unknown or used code");
            };
            if expires < now() {
                return bad("code expired");
            }
            if cid != client_id || ruri != redirect {
                return bad("code was issued to a different client");
            }
            if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge {
                return bad("PKCE verifier does not match");
            }
            if !a.allowed(&login) {
                return bad("not allowed");
            }
            match a.issue_tokens(&cid, &login) {
                Ok(v) => Json(v).into_response(),
                Err(e) => oauth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server_error",
                    &e.to_string(),
                ),
            }
        }
        "refresh_token" => {
            let Some(rt) = t.refresh_token else {
                return bad("refresh_token is required");
            };
            let row: Option<(String, String)> = {
                let db = a.inner.db.lock().unwrap();
                let row = db
                    .query_row(
                        "SELECT client_id, login FROM tokens WHERE hash = ?1 AND kind = 'refresh' AND expires > ?2",
                        params![hash(&rt), now()],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .ok()
                    .flatten();
                let _ = db.execute("DELETE FROM tokens WHERE hash = ?1", params![hash(&rt)]);
                row
            };
            let Some((cid, login)) = row else {
                return bad("unknown or expired refresh token");
            };
            if !client_id.is_empty() && cid != client_id {
                return bad("refresh token was issued to a different client");
            }
            if !a.allowed(&login) {
                return bad("not allowed");
            }
            match a.issue_tokens(&cid, &login) {
                Ok(v) => Json(v).into_response(),
                Err(e) => oauth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server_error",
                    &e.to_string(),
                ),
            }
        }
        _ => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "use authorization_code or refresh_token",
        ),
    }
}

/// Gate for `/mcp`: a live bearer token or a 401 that tells the client where to sign in.
pub async fn require_token(State(a): State<Auth>, mut req: Request, next: Next) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);
    match token.and_then(|t| a.check_bearer(&t)) {
        Some(login) => {
            req.extensions_mut().insert(Caller(login));
            next.run(req).await
        }
        None => {
            let mut res = (StatusCode::UNAUTHORIZED, "sign in required").into_response();
            let v = format!("Bearer resource_metadata=\"{}\"", a.resource_metadata_url());
            res.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_str(&v).unwrap());
            res
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeGithub(&'static str);
    impl GithubIdentity for FakeGithub {
        fn authorize_url(&self, state: &str, _cb: &str) -> String {
            format!("https://github.test/authorize?state={state}")
        }
        fn login_for_code(&self, _code: &str, _cb: &str) -> anyhow::Result<String> {
            Ok(self.0.to_string())
        }
    }

    fn auth(login: &'static str) -> (tempfile::TempDir, Auth) {
        let d = tempfile::tempdir().unwrap();
        let a = Auth::open(
            &d.path().join("auth.db"),
            "https://vault.test",
            &["Ada".into()],
            Box::new(FakeGithub(login)),
        )
        .unwrap();
        (d, a)
    }

    fn pkce() -> (String, String) {
        let verifier = random_token();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        (verifier, challenge)
    }

    async fn body_json(r: Response) -> serde_json::Value {
        let b = axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap();
        serde_json::from_slice(&b).unwrap()
    }

    /// Runs register → authorize → callback and returns (client_id, code).
    async fn sign_in(a: &Auth, challenge: &str) -> (String, Option<String>, StatusCode) {
        let r = register(
            State(a.clone()),
            Json(RegisterReq {
                redirect_uris: vec!["https://client.test/cb".into()],
                client_name: Some("t".into()),
            }),
        )
        .await;
        let cid = body_json(r).await["client_id"]
            .as_str()
            .unwrap()
            .to_string();
        let r = authorize(
            State(a.clone()),
            Query(AuthorizeReq {
                response_type: "code".into(),
                client_id: cid.clone(),
                redirect_uri: "https://client.test/cb".into(),
                code_challenge: Some(challenge.into()),
                code_challenge_method: Some("S256".into()),
                state: Some("xyz".into()),
            }),
        )
        .await;
        let loc = r.headers()[header::LOCATION].to_str().unwrap().to_string();
        let gh_state = loc.split("state=").nth(1).unwrap().to_string();
        let r = callback(
            State(a.clone()),
            Query(CallbackReq {
                code: Some("gh".into()),
                state: gh_state,
                error: None,
            }),
        )
        .await;
        let status = r.status();
        let code = r.headers().get(header::LOCATION).map(|l| {
            let u = url::Url::parse(l.to_str().unwrap()).unwrap();
            assert_eq!(
                u.query_pairs().find(|(k, _)| k == "state").unwrap().1,
                "xyz"
            );
            u.query_pairs()
                .find(|(k, _)| k == "code")
                .unwrap()
                .1
                .to_string()
        });
        (cid, code, status)
    }

    fn exchange(cid: &str, code: &str, verifier: &str) -> TokenReq {
        TokenReq {
            grant_type: "authorization_code".into(),
            code: Some(code.into()),
            redirect_uri: Some("https://client.test/cb".into()),
            client_id: Some(cid.into()),
            code_verifier: Some(verifier.into()),
            refresh_token: None,
        }
    }

    #[tokio::test]
    async fn allowed_login_gets_a_working_token_and_refresh_rotates_it() {
        let (_d, a) = auth("ada");
        let (verifier, challenge) = pkce();
        let (cid, code, _) = sign_in(&a, &challenge).await;
        let r = token(
            State(a.clone()),
            Form(exchange(&cid, &code.unwrap(), &verifier)),
        )
        .await;
        assert_eq!(r.status(), StatusCode::OK);
        let t = body_json(r).await;
        let access = t["access_token"].as_str().unwrap();
        assert_eq!(a.check_bearer(access).as_deref(), Some("ada"));
        let refresh = t["refresh_token"].as_str().unwrap().to_string();
        let r = token(
            State(a.clone()),
            Form(TokenReq {
                grant_type: "refresh_token".into(),
                refresh_token: Some(refresh.clone()),
                client_id: Some(cid.clone()),
                code: None,
                redirect_uri: None,
                code_verifier: None,
            }),
        )
        .await;
        assert_eq!(r.status(), StatusCode::OK);
        let r = token(
            State(a.clone()),
            Form(TokenReq {
                grant_type: "refresh_token".into(),
                refresh_token: Some(refresh),
                client_id: Some(cid),
                code: None,
                redirect_uri: None,
                code_verifier: None,
            }),
        )
        .await;
        assert_eq!(
            r.status(),
            StatusCode::BAD_REQUEST,
            "a refresh token works once"
        );
    }

    #[tokio::test]
    async fn login_not_on_the_allowlist_gets_no_code() {
        let (_d, a) = auth("someone-else");
        let (_, challenge) = pkce();
        let (_, code, status) = sign_in(&a, &challenge).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(code.is_none());
    }

    #[tokio::test]
    async fn wrong_pkce_verifier_and_reused_codes_are_refused() {
        let (_d, a) = auth("ada");
        let (verifier, challenge) = pkce();
        let (cid, code, _) = sign_in(&a, &challenge).await;
        let code = code.unwrap();
        let r = token(
            State(a.clone()),
            Form(exchange(&cid, &code, "wrong-verifier")),
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r = token(State(a.clone()), Form(exchange(&cid, &code, &verifier))).await;
        assert_eq!(
            r.status(),
            StatusCode::BAD_REQUEST,
            "the failed attempt burned the code"
        );
    }

    #[tokio::test]
    async fn unregistered_redirects_and_missing_pkce_are_refused() {
        let (_d, a) = auth("ada");
        let r = register(
            State(a.clone()),
            Json(RegisterReq {
                redirect_uris: vec!["http://evil.test/cb".into()],
                client_name: None,
            }),
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r = register(
            State(a.clone()),
            Json(RegisterReq {
                redirect_uris: vec!["https://client.test/cb".into()],
                client_name: None,
            }),
        )
        .await;
        let cid = body_json(r).await["client_id"]
            .as_str()
            .unwrap()
            .to_string();
        let r = authorize(
            State(a.clone()),
            Query(AuthorizeReq {
                response_type: "code".into(),
                client_id: cid.clone(),
                redirect_uri: "https://other.test/cb".into(),
                code_challenge: Some("x".into()),
                code_challenge_method: Some("S256".into()),
                state: None,
            }),
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r = authorize(
            State(a.clone()),
            Query(AuthorizeReq {
                response_type: "code".into(),
                client_id: cid,
                redirect_uri: "https://client.test/cb".into(),
                code_challenge: None,
                code_challenge_method: None,
                state: None,
            }),
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn garbage_and_expired_bearers_are_refused() {
        let (_d, a) = auth("ada");
        assert!(a.check_bearer("nope").is_none());
        let t = a.issue_tokens("c", "ada").unwrap();
        let access = t["access_token"].as_str().unwrap();
        a.inner
            .db
            .lock()
            .unwrap()
            .execute("UPDATE tokens SET expires = 0", [])
            .unwrap();
        assert!(a.check_bearer(access).is_none());
    }
}
