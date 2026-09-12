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
    /// Whose decision it is. A team decision outranks a personal one when they
    /// disagree, because the point of writing it down was to settle it for everyone.
    pub scope: Scope,
}

impl Decision {
    /// The one-line form an agent reads.
    pub fn line(&self) -> String {
        format!(
            "{}{}: {}",
            self.scope.label(),
            if self.against { "ruled out" } else { "decided" },
            self.text
        )
    }
}

/// Where a decision lives, and who it belongs to.
///
/// The whole scaling story is in this enum. A personal decision sits in the vault
/// and is one person's. A team decision sits in `<repo>/DECISIONS.md`, which means
/// git distributes it: one person rules something out, commits, and every
/// teammate's agent knows by their next pull. No server, no account, no sync
/// service, and it works for a team of two or two hundred because the mechanism is
/// the one they already use for everything else.
///
/// The team file is deliberately not named after this tool. A format called
/// `.glide/` is one no other tool will ever read, and a decision record is only
/// worth writing if whatever agent the next person runs can read it too. The
/// format is specified in SPEC.md and glide is one implementation of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Personal,
    Team,
}

impl Scope {
    pub fn label(&self) -> &'static str {
        match self {
            Scope::Personal => "",
            Scope::Team => "team: ",
        }
    }
}

pub fn note_path(root: &Path) -> PathBuf {
    root.join("Decisions.md")
}

/// The file this tool writes team decisions to.
pub const TEAM_FILE: &str = "DECISIONS.md";

/// Read as well as written, for teams that would rather not have it at the root.
/// Tools should accept both and write the first.
pub const TEAM_FILE_ALT: &str = ".glide/decisions.md";

/// Where to write a team decision: the repository root, beside README.md.
pub fn team_path(repo_root: &Path) -> PathBuf {
    repo_root.join(TEAM_FILE)
}

/// Where to read one from, which is whichever of the two a repo actually has.
///
/// A repo that already keeps the file tucked away keeps working; a repo with
/// neither gets the root path, so the first `--team` decision creates the
/// discoverable one.
pub fn team_path_for_read(repo_root: &Path) -> PathBuf {
    let root = repo_root.join(TEAM_FILE);
    if root.exists() {
        return root;
    }
    let alt = repo_root.join(TEAM_FILE_ALT);
    if alt.exists() {
        return alt;
    }
    root
}

/// Read the decisions note. A missing file is an empty list, not an error: nobody
/// has decided anything yet is a legitimate state and should not stop a session.
pub fn load(root: &Path) -> Result<Vec<Decision>> {
    load_from(&note_path(root), Scope::Personal)
}

/// The team's decisions, from a repository. Absent is empty, not an error: most
/// directories are not repositories and that must not stop a session.
pub fn load_team(repo_root: &Path) -> Result<Vec<Decision>> {
    load_from(&team_path_for_read(repo_root), Scope::Team)
}

fn load_from(path: &Path, scope: Scope) -> Result<Vec<Decision>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(parse_scoped(&fs::read_to_string(path)?, scope))
}

/// Team decisions first, then personal, newest last within each.
///
/// Order is the conflict rule made visible: when the two disagree the team's is
/// read first and the personal one reads as the exception it is.
pub fn merge(team: Vec<Decision>, personal: Vec<Decision>) -> Vec<Decision> {
    let mut seen: Vec<String> = team.iter().map(key).collect();
    let mut out = team;
    for d in personal {
        let k = key(&d);
        if seen.contains(&k) {
            continue;
        }
        seen.push(k);
        out.push(d);
    }
    out
}

/// Someone who settles a thing personally and then again for the team should not
/// spend two of an agent's eight slots saying it once.
pub fn key(d: &Decision) -> String {
    format!("{}|{}", d.against, d.text.trim().to_lowercase())
}

pub fn parse(raw: &str) -> Vec<Decision> {
    parse_scoped(raw, Scope::Personal)
}

pub fn parse_scoped(raw: &str, scope: Scope) -> Vec<Decision> {
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
                scope,
            });
        } else if let Some(x) = rest.strip_prefix("decided: ") {
            out.push(Decision {
                text: x.trim().to_string(),
                against: false,
                on,
                scope,
            });
        }
    }
    out
}

/// Append one decision, creating the note if it is not there.
pub fn add(root: &Path, text: &str, against: bool, on: NaiveDate) -> Result<Decision> {
    add_at(&note_path(root), text, against, on, Scope::Personal)
}

/// Record a team decision into the repository, where git will carry it.
pub fn add_team(repo_root: &Path, text: &str, against: bool, on: NaiveDate) -> Result<Decision> {
    add_at(
        &team_path_for_read(repo_root),
        text,
        against,
        on,
        Scope::Team,
    )
}

fn add_at(path: &Path, text: &str, against: bool, on: NaiveDate, scope: Scope) -> Result<Decision> {
    let d = Decision {
        text: text.trim().to_string(),
        against,
        on: on.format("%Y-%m-%d").to_string(),
        scope,
    };
    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        "# Decisions\n\nWhat has been settled, so nothing settled gets proposed again.\n\n"
            .to_string()
    };
    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str(&format!(
        "- {} {}: {}\n",
        d.on,
        if d.against { "ruled out" } else { "decided" },
        d.text
    ));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, raw)?;
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

    #[test]
    fn recent_of_nothing_is_nothing() {
        assert!(recent(&[], 8).is_empty());
        assert!(recent(&parse(NOTE), 0).is_empty());
    }

    #[test]
    fn a_team_decision_says_it_is_the_teams() {
        let d = &parse_scoped("- 2026-01-01 ruled out: Mongo", Scope::Team)[0];
        assert_eq!(d.line(), "team: ruled out: Mongo");
        assert_eq!(d.scope, Scope::Team);
    }

    #[test]
    fn a_team_decision_written_to_the_repo_reads_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let on = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        add_team(dir.path(), "Postgres", false, on).unwrap();
        add_team(dir.path(), "Mongo", true, on).unwrap();
        let back = load_team(dir.path()).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].line(), "team: ruled out: Mongo");
        assert!(team_path(dir.path()).ends_with("DECISIONS.md"));
    }

    #[test]
    fn a_team_decision_is_not_written_into_the_personal_note() {
        // A vault and a repository are different folders, which is what keeps
        // `DECISIONS.md` and the vault's `Decisions.md` from being one file on a
        // case-insensitive disk.
        let vault = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let on = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        add_team(repo.path(), "Postgres", false, on).unwrap();
        assert!(load(vault.path()).unwrap().is_empty());
        assert_eq!(load_team(repo.path()).unwrap().len(), 1);
    }

    #[test]
    fn the_team_is_read_first_so_it_wins_when_the_two_disagree() {
        let team = parse_scoped("- 2026-01-01 ruled out: Mongo", Scope::Team);
        let mine = parse_scoped("- 2026-01-02 decided: Mongo actually", Scope::Personal);
        let merged = merge(team, mine);
        assert_eq!(merged[0].line(), "team: ruled out: Mongo");
        assert_eq!(merged[1].line(), "decided: Mongo actually");
    }

    #[test]
    fn settling_the_same_thing_twice_is_listed_once() {
        let team = parse_scoped("- 2026-01-01 ruled out: Mongo", Scope::Team);
        let mine = parse_scoped("- 2026-01-02 ruled out:  mongo ", Scope::Personal);
        let merged = merge(team, mine);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].scope, Scope::Team);
    }

    #[test]
    fn a_repo_that_already_tucked_the_file_away_keeps_using_it() {
        let dir = tempfile::tempdir().unwrap();
        let alt = dir.path().join(TEAM_FILE_ALT);
        std::fs::create_dir_all(alt.parent().unwrap()).unwrap();
        std::fs::write(&alt, "- 2026-01-01 ruled out: Mongo\n").unwrap();

        assert_eq!(load_team(dir.path()).unwrap().len(), 1);
        add_team(
            dir.path(),
            "Postgres",
            false,
            chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
        )
        .unwrap();
        assert_eq!(load_team(dir.path()).unwrap().len(), 2);
        assert!(
            !dir.path().join(TEAM_FILE).exists(),
            "must not start a second file"
        );
    }

    #[test]
    fn a_repo_with_neither_file_gets_the_discoverable_one() {
        let dir = tempfile::tempdir().unwrap();
        add_team(
            dir.path(),
            "Postgres",
            false,
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        )
        .unwrap();
        assert!(dir.path().join(TEAM_FILE).exists());
        assert!(!dir.path().join(TEAM_FILE_ALT).exists());
    }

    #[test]
    fn a_missing_team_file_is_no_decisions_rather_than_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_team(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn the_scope_label_is_not_written_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let on = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        add_team(dir.path(), "Postgres", false, on).unwrap();
        let raw = std::fs::read_to_string(team_path_for_read(dir.path())).unwrap();
        assert!(raw.contains("- 2026-01-01 decided: Postgres"), "{raw}");
        assert!(!raw.contains("team:"), "{raw}");
    }
}
