//! Decisions: the thing an agent keeps re-proposing because nobody told it.
//!
//! Every other tool in this category injects what HAPPENED. Session memory,
//! compressed command output, retrieved facts: all of it is recall. None of it
//! answers what this person decided, because none of them have a place where a
//! person writes a decision down.
//!
//! That gap is expensive in the one unit that matters. A session re-reads its whole
//! context on every turn, so the cost of an agent proposing an approach that was
//! ruled out last week is not the sentence it wastes, it is the turns spent
//! proposing it, hearing no, and picking again. A decision costs about twenty tokens
//! to state and saves the turns that would have re-litigated it.
//!
//! Decisions outlive a day, so they live in their own note rather than in the daily
//! one, in the same vault, editable by hand like everything else here.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::Result;

/// One decision, and whether it was a choice or a rejection.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub text: String,
    /// True when this records something ruled OUT, which is the half agents repeat.
    pub against: bool,
    pub on: String,
}

impl Decision {
    /// The one-line form an agent reads.
    pub fn line(&self) -> String {
        if self.against {
            format!("ruled out: {}", self.text)
        } else {
            format!("decided: {}", self.text)
        }
    }
}

pub fn note_path(root: &Path) -> PathBuf {
    root.join("Decisions.md")
}

/// Read the decisions note. A missing file is an empty list, not an error: nobody
/// has decided anything yet is a legitimate state and should not stop a session.
pub fn load(root: &Path) -> Result<Vec<Decision>> {
    let path = note_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(parse(&fs::read_to_string(path)?))
}

pub fn parse(raw: &str) -> Vec<Decision> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim_start();
        let body = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            Some(b) => b.trim(),
            None => continue,
        };
        // `YYYY-MM-DD` prefix is written by this tool and is not part of the text.
        let (on, rest) = match body.split_once(' ') {
            Some((d, r)) if d.len() == 10 && d.chars().filter(|c| *c == '-').count() == 2 => {
                (d.to_string(), r.trim())
            }
            _ => (String::new(), body),
        };
        if let Some(x) = rest.strip_prefix("ruled out: ") {
            out.push(Decision {
                text: x.trim().to_string(),
                against: true,
                on,
            });
        } else if let Some(x) = rest.strip_prefix("decided: ") {
            out.push(Decision {
                text: x.trim().to_string(),
                against: false,
                on,
            });
        }
    }
    out
}

/// Append one decision, creating the note if it is not there.
pub fn add(root: &Path, text: &str, against: bool, on: NaiveDate) -> Result<Decision> {
    let path = note_path(root);
    let d = Decision {
        text: text.trim().to_string(),
        against,
        on: on.format("%Y-%m-%d").to_string(),
    };
    let mut raw = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        "# Decisions\n\nWhat has been settled, so nothing settled gets proposed again.\n\n"
            .to_string()
    };
    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str(&format!("- {} {}\n", d.on, d.line()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, raw)?;
    Ok(d)
}

/// The most recent `n`, newest last so the freshest sits closest to the question.
///
/// Position matters: a fact buried mid-context is likelier to be missed than one at
/// either end, so the list is short and the newest decision is the last thing read.
pub fn recent(all: &[Decision], n: usize) -> Vec<&Decision> {
    let start = all.len().saturating_sub(n);
    all[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "# Decisions\n\nblurb\n\n- 2026-09-01 decided: postgres, the replication story matters\n- 2026-09-02 ruled out: mongo, schema churn already bit us\n- not a decision line\n";

    #[test]
    fn reads_both_kinds() {
        let d = parse(NOTE);
        assert_eq!(d.len(), 2);
        assert!(!d[0].against);
        assert!(d[1].against);
        assert_eq!(d[1].text, "mongo, schema churn already bit us");
        assert_eq!(d[1].on, "2026-09-02");
    }

    #[test]
    fn a_rejection_says_so_in_one_line() {
        // The half that matters: an agent needs to know what NOT to propose, and
        // "ruled out" has to survive into the injected text unambiguously.
        let d = parse(NOTE);
        assert_eq!(d[1].line(), "ruled out: mongo, schema churn already bit us");
    }

    #[test]
    fn ignores_prose_between_the_bullets() {
        assert_eq!(
            parse("# x\n\nsome words\n- and a bullet with no verb\n").len(),
            0
        );
    }

    #[test]
    fn recent_keeps_the_newest_last() {
        let d = parse(NOTE);
        let r = recent(&d, 1);
        assert_eq!(r.len(), 1);
        assert!(r[0].against, "the newest is the rejection");
    }

    #[test]
    fn recent_does_not_panic_when_there_are_fewer_than_asked_for() {
        assert_eq!(recent(&parse(NOTE), 99).len(), 2);
    }
}
