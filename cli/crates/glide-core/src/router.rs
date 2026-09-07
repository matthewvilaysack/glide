use serde::Serialize;

/// One-shot router. `glide -p "<question>"` runs this to pick a verb.
/// v1 is a simple keyword router; v2 hands the question to glide-llm.
#[derive(Debug, Clone, Serialize)]
pub enum RouteDecision {
    WhoOwns { query: String },
    Plan { person: String },
    Request { permission: String },
    FrictionDigest,
    Unrouted { reason: String },
}

pub fn route_one_shot(prompt: &str) -> RouteDecision {
    let lower = prompt.to_ascii_lowercase();
    if lower.starts_with("who owns") || lower.contains("ownership") || lower.contains("owner of") {
        let q = prompt
            .split_once("owns")
            .map(|x| x.1)
            .or_else(|| prompt.split_once("owner of").map(|x| x.1))
            .map(str::trim)
            .map(|s| s.trim_matches(|c: char| c == '?' || c == '.').to_string())
            .unwrap_or_else(|| prompt.to_string());
        return RouteDecision::WhoOwns { query: q };
    }
    if lower.starts_with("plan for") || lower.contains("day-1 plan") || lower.contains("day 1 plan")
    {
        let person = prompt
            .split_once("for")
            .map(|x| x.1)
            .map(str::trim)
            .unwrap_or("(unspecified)")
            .to_string();
        return RouteDecision::Plan { person };
    }
    if lower.contains("access to")
        || lower.contains("request access")
        || lower.contains("permission")
    {
        return RouteDecision::Request {
            permission: extract_permission(prompt),
        };
    }
    if lower.contains("friction") || lower.contains("digest") {
        return RouteDecision::FrictionDigest;
    }
    RouteDecision::Unrouted {
        reason: "no verb matched; v2 will route via LLM when [llm].enabled = true".into(),
    }
}

/// Pull the thing being asked for out of the sentence, so "i need access to
/// pomelo" requests `pomelo` and not the whole sentence.
fn extract_permission(prompt: &str) -> String {
    const LEAD_INS: [&str; 3] = ["access to", "permission for", "permissions for"];

    let lower = prompt.to_ascii_lowercase();
    let tail = LEAD_INS
        .iter()
        .find_map(|marker| lower.find(marker).map(|i| &prompt[i + marker.len()..]))
        .unwrap_or(prompt);

    let cleaned = tail
        .trim()
        .trim_matches(|c: char| c == '?' || c == '.' || c == '`');
    let cleaned = cleaned
        .strip_prefix("the ")
        .or_else(|| cleaned.strip_prefix("a "))
        .unwrap_or(cleaned);

    if cleaned.is_empty() {
        prompt.trim().to_string()
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permission_of(prompt: &str) -> String {
        match route_one_shot(prompt) {
            RouteDecision::Request { permission } => permission,
            other => panic!("expected a request route for {prompt:?}, got {other:?}"),
        }
    }

    #[test]
    fn a_request_names_the_tool_not_the_sentence() {
        assert_eq!(permission_of("i need access to pomelo"), "pomelo");
        assert_eq!(
            permission_of("request access to the staging-db"),
            "staging-db"
        );
        assert_eq!(
            permission_of("can i get permission for admin:billing?"),
            "admin:billing"
        );
    }

    #[test]
    fn a_bare_permission_word_still_routes_to_request() {
        assert_eq!(permission_of("permission"), "permission");
    }

    #[test]
    fn ownership_questions_still_route_to_who_owns() {
        match route_one_shot("who owns billing/charge.py?") {
            RouteDecision::WhoOwns { query } => assert_eq!(query, "billing/charge.py"),
            other => panic!("expected who-owns, got {other:?}"),
        }
    }
}
