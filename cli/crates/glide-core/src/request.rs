use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RequestResponse {
    pub permission: String,
    pub for_person: Option<String>,
    pub draft_message: String,
    pub channels: Vec<String>,
    pub status: &'static str,
}

/// v1 draft: produce a Slack-ready message template. v2 wires actual send.
pub fn request(permission: &str, for_person: Option<String>) -> Result<RequestResponse> {
    let who = for_person.clone().unwrap_or_else(|| "the new hire".into());
    let msg = format!(
        "Hi team — could we get `{}` access for {}? Blocking ramp; happy to provide context.",
        permission, who
    );
    Ok(RequestResponse {
        permission: permission.into(),
        for_person,
        draft_message: msg,
        channels: vec!["slack:#access-requests".into(), "jira:ACCESS-NEW".into()],
        status: "draft (no network send in v1; pipe to `pbcopy` or your sender of choice)",
    })
}
