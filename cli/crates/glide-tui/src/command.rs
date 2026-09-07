//! Verb-first parsing. Commands are commands; anything else is a question and
//! falls through to `route_one_shot`.
//!
//! This has to run *before* the router, because the router's keyword matching
//! would swallow several of these: `friction log needs staging access` contains
//! "friction", so it would land on the digest rather than logging an event.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Doctor,
    IndexShow,
    IndexBuild,
    ConfigList,
    ConfigGet(String),
    ConfigSet(String, String),
    ConfigPaths,
    Models,
    ToolsStatus,
    ToolsInstall(String),
    FrictionLog {
        subject: String,
        category: String,
        severity: u8,
    },
    FrictionDigest {
        since_days: i64,
    },
    WhoOwns(String),
    Plan(String),
    Request(String),
    /// `focus`, `focus set <text>`, `focus done <text>`, `focus capture <text>`, `focus log <text>`.
    Focus {
        action: String,
        text: String,
    },
}

/// Parse a verb-first line. `None` means "this is a question, route it".
pub fn parse(line: &str) -> Option<Cmd> {
    let line = line.trim();
    let line = line.strip_prefix("glide ").unwrap_or(line).trim();
    let mut parts = line.split_whitespace();
    let verb = parts.next()?;
    let rest: Vec<&str> = parts.collect();

    match verb {
        "doctor" => Some(Cmd::Doctor),

        // Priorities live in the vault, not the graph. Bare `focus` shows.
        "focus" => match rest.first().copied() {
            None | Some("show") => Some(Cmd::Focus {
                action: "show".into(),
                text: String::new(),
            }),
            Some(a @ ("set" | "done" | "capture" | "log" | "today")) => Some(Cmd::Focus {
                action: a.to_string(),
                text: rest[1..].join(" "),
            }),
            _ => None,
        },
        "models" => Some(Cmd::Models),

        // Bare `index` shows. Building drops and rewrites four tables, so it
        // never happens by accident.
        "index" => match rest.first().copied() {
            None | Some("show") => Some(Cmd::IndexShow),
            Some("build") => Some(Cmd::IndexBuild),
            _ => None,
        },

        "config" => match rest.first().copied() {
            None | Some("list") => Some(Cmd::ConfigList),
            Some("paths") => Some(Cmd::ConfigPaths),
            Some("get") => rest.get(1).map(|k| Cmd::ConfigGet(k.to_string())),
            Some("set") => match (rest.get(1), rest.get(2)) {
                (Some(k), Some(_)) => Some(Cmd::ConfigSet(k.to_string(), rest[2..].join(" "))),
                _ => None,
            },
            _ => None,
        },

        "tools" => match rest.first().copied() {
            None | Some("status") => Some(Cmd::ToolsStatus),
            Some("install") => Some(Cmd::ToolsInstall(
                rest.get(1).copied().unwrap_or("repomix").to_string(),
            )),
            _ => None,
        },

        "friction" => match rest.first().copied() {
            Some("log") => Some(parse_friction_log(&rest[1..])),
            None | Some("digest") => Some(Cmd::FrictionDigest {
                since_days: rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(7),
            }),
            _ => None,
        },

        "who-owns" | "who-own" => (!rest.is_empty()).then(|| Cmd::WhoOwns(rest.join(" "))),
        "plan" => {
            // `plan for priya` and `plan priya` are the same ask.
            let words = if rest.first() == Some(&"for") {
                &rest[1..]
            } else {
                &rest[..]
            };
            (!words.is_empty()).then(|| Cmd::Plan(words.join(" ")))
        }
        "request" => (!rest.is_empty()).then(|| Cmd::Request(rest.join(" "))),

        _ => None,
    }
}

/// `friction log <subject...> [--severity N] [--category C]`.
fn parse_friction_log(args: &[&str]) -> Cmd {
    let mut subject = Vec::new();
    let mut category = "access".to_string();
    let mut severity = 3u8;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--severity" | "-s" => {
                if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                    severity = v;
                }
                i += 2;
            }
            "--category" | "-c" => {
                if let Some(v) = args.get(i + 1) {
                    category = v.to_string();
                }
                i += 2;
            }
            word => {
                subject.push(word);
                i += 1;
            }
        }
    }

    Cmd::FrictionLog {
        subject: subject.join(" "),
        category,
        severity: severity.clamp(1, 5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_verbs_keep_the_whole_text() {
        assert_eq!(
            parse("focus"),
            Some(Cmd::Focus {
                action: "show".into(),
                text: String::new()
            })
        );
        assert_eq!(
            parse("focus set ship portal tests"),
            Some(Cmd::Focus {
                action: "set".into(),
                text: "ship portal tests".into()
            })
        );
        assert_eq!(
            parse("focus log fixed the build"),
            Some(Cmd::Focus {
                action: "log".into(),
                text: "fixed the build".into()
            })
        );
        assert_eq!(parse("focus nope"), None);
    }

    #[test]
    fn bare_verbs_pick_the_read_only_subcommand() {
        assert_eq!(parse("index"), Some(Cmd::IndexShow));
        assert_eq!(parse("config"), Some(Cmd::ConfigList));
        assert_eq!(parse("tools"), Some(Cmd::ToolsStatus));
        assert_eq!(
            parse("friction"),
            Some(Cmd::FrictionDigest { since_days: 7 })
        );
    }

    #[test]
    fn building_the_index_is_never_implicit() {
        // The only spelling that wipes and rebuilds the graph.
        assert_eq!(parse("index build"), Some(Cmd::IndexBuild));
        assert_ne!(parse("index"), Some(Cmd::IndexBuild));
        assert_ne!(parse("index show"), Some(Cmd::IndexBuild));
    }

    #[test]
    fn friction_log_beats_the_routers_friction_keyword() {
        assert_eq!(
            parse("friction log blocked on staging db"),
            Some(Cmd::FrictionLog {
                subject: "blocked on staging db".into(),
                category: "access".into(),
                severity: 3,
            })
        );
    }

    #[test]
    fn friction_log_flags_are_pulled_out_of_the_subject() {
        assert_eq!(
            parse("friction log no docs --severity 5 --category doc-gap"),
            Some(Cmd::FrictionLog {
                subject: "no docs".into(),
                category: "doc-gap".into(),
                severity: 5,
            })
        );
    }

    #[test]
    fn an_out_of_range_severity_is_clamped_not_rejected() {
        match parse("friction log x --severity 9") {
            Some(Cmd::FrictionLog { severity, .. }) => assert_eq!(severity, 5),
            other => panic!("expected a friction log, got {other:?}"),
        }
    }

    #[test]
    fn config_set_keeps_multi_word_values_whole() {
        assert_eq!(
            parse("config set llm.default_provider anthropic"),
            Some(Cmd::ConfigSet(
                "llm.default_provider".into(),
                "anthropic".into()
            ))
        );
        assert_eq!(
            parse("config set a.b two words"),
            Some(Cmd::ConfigSet("a.b".into(), "two words".into()))
        );
    }

    #[test]
    fn an_incomplete_config_command_falls_through_to_the_router() {
        assert_eq!(parse("config get"), None);
        assert_eq!(parse("config set onlykey"), None);
    }

    #[test]
    fn plan_accepts_both_spellings() {
        assert_eq!(parse("plan priya"), Some(Cmd::Plan("priya".into())));
        assert_eq!(parse("plan for priya"), Some(Cmd::Plan("priya".into())));
    }

    #[test]
    fn the_glide_prefix_is_optional() {
        assert_eq!(parse("glide doctor"), Some(Cmd::Doctor));
        assert_eq!(parse("glide index build"), Some(Cmd::IndexBuild));
    }

    #[test]
    fn questions_are_left_for_the_router() {
        assert_eq!(parse("who owns billing/charge.py"), None);
        assert_eq!(parse("i need access to pomelo"), None);
        assert_eq!(parse(""), None);
    }
}
