//! The vault contract: one Markdown daily note per day, in a folder the user
//! already syncs (Obsidian over iCloud, or any folder sync). Glide reads and
//! edits four `##` sections of today's note and never touches anything else.
//!
//! - `## Focus`: the day's priorities as bullets; the one tagged `#now` is the
//!   current focus.
//! - `## Tasks`: checkbox bullets, sub-headings allowed.
//! - `## Record`: what actually happened, appended as `- HH:MM text`.
//! - `## Notes`: quick captures, appended as `- text`.

use std::path::PathBuf;

use chrono::{Local, NaiveDate};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error(
        "vault path is not set; run `glide config set vault.path <folder>` or set GLIDE_VAULT_PATH"
    )]
    NoPath,
    #[error("vault folder does not exist: {0}")]
    Missing(PathBuf),
    #[error("no line in Focus or Tasks matches {0:?}")]
    NoMatch(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VaultError>;

const NOW_TAG: &str = "#now";
const SECTIONS: [&str; 4] = ["Focus", "Tasks", "Record", "Notes"];

/// Where the notes live and how a day maps to a file.
#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
    /// Relative pattern with `{date}` for `YYYY-MM-DD`, e.g. `Daily Notes/{date}.md`.
    pub daily_note_pattern: String,
}

impl Vault {
    pub fn new(path: &str, daily_note_pattern: &str) -> Result<Self> {
        if path.trim().is_empty() {
            return Err(VaultError::NoPath);
        }
        let root = expand_home(path);
        if !root.is_dir() {
            return Err(VaultError::Missing(root));
        }
        Ok(Vault {
            root,
            daily_note_pattern: daily_note_pattern.to_string(),
        })
    }

    pub fn today() -> NaiveDate {
        Local::now().date_naive()
    }

    pub fn note_path(&self, date: NaiveDate) -> PathBuf {
        let rel = self
            .daily_note_pattern
            .replace("{date}", &date.format("%Y-%m-%d").to_string());
        self.root.join(rel)
    }

    /// Load the note for `date`, or an empty skeleton if it does not exist yet.
    pub fn load(&self, date: NaiveDate) -> Result<Daily> {
        let path = self.note_path(date);
        let raw = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            skeleton(date)
        };
        Ok(Daily { path, date, raw })
    }
}

fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(base) = directories::BaseDirs::new() {
            return base.home_dir().join(rest);
        }
    }
    PathBuf::from(p)
}

fn skeleton(date: NaiveDate) -> String {
    let mut s = format!("# {}\n", date.format("%Y-%m-%d"));
    for name in SECTIONS {
        s.push_str(&format!("\n## {name}\n"));
    }
    s
}

/// One bullet in Focus or Tasks.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Item {
    pub text: String,
    pub done: bool,
    pub now: bool,
}

/// What the focus line and the `today` tool report.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub date: String,
    pub path: String,
    pub exists: bool,
    pub now: Option<String>,
    pub focus: Vec<Item>,
    pub tasks_open: usize,
    pub tasks_done: usize,
    pub record_entries: usize,
    pub notes: usize,
}

impl Snapshot {
    /// The one-liner for a tmux status bar or shell prompt.
    pub fn line(&self) -> String {
        let mut parts = Vec::new();
        parts.push(match &self.now {
            Some(n) => format!("▶ {n}"),
            None => "▶ no focus set".to_string(),
        });
        if !self.focus.is_empty() {
            let done = self.focus.iter().filter(|i| i.done).count();
            parts.push(format!("focus {done}/{}", self.focus.len()));
        }
        if self.tasks_open > 0 {
            parts.push(format!("{} open", self.tasks_open));
        }
        if self.record_entries > 0 {
            parts.push(format!("{} logged", self.record_entries));
        }
        parts.join(" · ")
    }
}

/// A loaded daily note. Edits work on the raw text so everything outside the
/// four sections survives byte for byte.
#[derive(Debug, Clone)]
pub struct Daily {
    pub path: PathBuf,
    pub date: NaiveDate,
    pub raw: String,
}

impl Daily {
    pub fn parse(date: NaiveDate, raw: &str) -> Self {
        Daily {
            path: PathBuf::new(),
            date,
            raw: raw.to_string(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let focus = self.focus_items();
        let tasks = self.task_items();
        Snapshot {
            date: self.date.format("%Y-%m-%d").to_string(),
            path: self.path.display().to_string(),
            exists: self.path.exists(),
            now: focus
                .iter()
                .find(|i| i.now && !i.done)
                .map(|i| i.text.clone()),
            tasks_open: tasks.iter().filter(|i| !i.done).count(),
            tasks_done: tasks.iter().filter(|i| i.done).count(),
            record_entries: self
                .section_lines("Record")
                .iter()
                .filter(|l| is_bullet(l))
                .count(),
            notes: self
                .section_lines("Notes")
                .iter()
                .filter(|l| is_bullet(l))
                .count(),
            focus,
        }
    }

    pub fn focus_items(&self) -> Vec<Item> {
        self.section_lines("Focus")
            .iter()
            .filter_map(|l| parse_item(l))
            .collect()
    }

    pub fn task_items(&self) -> Vec<Item> {
        self.section_lines("Tasks")
            .iter()
            .filter_map(|l| parse_item(l))
            .filter(|i| !i.text.is_empty())
            .collect()
    }

    /// Make `text` the current focus. Matches an existing Focus bullet
    /// case-insensitively by substring, else adds a new one. Returns the text
    /// of the bullet that now carries the tag.
    pub fn set_now(&mut self, text: &str) -> String {
        let (start, end) = self.section_span("Focus");
        let mut lines: Vec<String> = self.raw.lines().map(String::from).collect();
        for l in lines[start..end].iter_mut() {
            *l = strip_now(l);
        }
        let needle = text.trim().to_lowercase();
        let hit = (start..end).find(|&i| {
            parse_item(&lines[i])
                .map(|it| it.text.to_lowercase().contains(&needle))
                .unwrap_or(false)
        });
        let chosen = match hit {
            Some(i) => {
                lines[i] = format!("{} {NOW_TAG}", lines[i].trim_end());
                parse_item(&lines[i]).map(|it| it.text).unwrap_or_default()
            }
            None => {
                let at = insert_point(&lines, start, end);
                lines.insert(at, format!("- [ ] {} {NOW_TAG}", text.trim()));
                text.trim().to_string()
            }
        };
        self.raw = join(lines);
        chosen
    }

    /// Check off the first Focus or Tasks bullet matching `text`.
    pub fn mark_done(&mut self, text: &str) -> Result<String> {
        let needle = text.trim().to_lowercase();
        let mut lines: Vec<String> = self.raw.lines().map(String::from).collect();
        for name in ["Focus", "Tasks"] {
            let (start, end) = self.section_span(name);
            if let Some(i) = (start..end).find(|&i| {
                parse_item(&lines[i])
                    .map(|it| !it.done && it.text.to_lowercase().contains(&needle))
                    .unwrap_or(false)
            }) {
                let l = strip_now(&lines[i]);
                lines[i] = match l.find("- [ ]") {
                    Some(p) => format!("{}- [x]{}", &l[..p], &l[p + 5..]),
                    None => l.replacen("- ", "- [x] ", 1),
                };
                let done = parse_item(&lines[i]).map(|it| it.text).unwrap_or_default();
                self.raw = join(lines);
                return Ok(done);
            }
        }
        Err(VaultError::NoMatch(text.to_string()))
    }

    /// Append a bullet to a section (creating the section if the note lacks it).
    pub fn append(&mut self, section: &str, line: &str) {
        let mut lines: Vec<String> = self.raw.lines().map(String::from).collect();
        let (start, end) = match self.find_section(&lines, section) {
            Some(span) => span,
            None => {
                if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                    lines.push(String::new());
                }
                lines.push(format!("## {section}"));
                (lines.len(), lines.len())
            }
        };
        let at = insert_point(&lines, start, end);
        lines.insert(at, format!("- {}", line.trim()));
        self.raw = join(lines);
    }

    pub fn capture(&mut self, text: &str) {
        self.append("Notes", text);
    }

    pub fn log(&mut self, text: &str) {
        let stamp = Local::now().format("%H:%M");
        self.append("Record", &format!("{stamp} {}", text.trim()));
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.raw)?;
        Ok(())
    }

    fn section_lines(&self, name: &str) -> Vec<String> {
        let lines: Vec<String> = self.raw.lines().map(String::from).collect();
        match self.find_section(&lines, name) {
            Some((s, e)) => lines[s..e].to_vec(),
            None => Vec::new(),
        }
    }

    /// (start, end) line indexes of the body after `## name`, up to the next
    /// top-level heading. `(0, 0)` when the section is absent.
    fn section_span(&self, name: &str) -> (usize, usize) {
        let lines: Vec<String> = self.raw.lines().map(String::from).collect();
        self.find_section(&lines, name).unwrap_or((0, 0))
    }

    fn find_section(&self, lines: &[String], name: &str) -> Option<(usize, usize)> {
        let want = format!("## {}", name.trim()).to_lowercase();
        let start = lines.iter().position(|l| l.trim().to_lowercase() == want)? + 1;
        let end = lines[start..]
            .iter()
            .position(|l| l.starts_with("## ") || l.starts_with("# "))
            .map(|off| start + off)
            .unwrap_or(lines.len());
        Some((start, end))
    }
}

/// Where a new bullet goes: after the last non-blank line of the section, so a
/// blank line before the next heading is preserved.
fn insert_point(lines: &[String], start: usize, end: usize) -> usize {
    let mut i = end;
    while i > start && lines[i - 1].trim().is_empty() {
        i -= 1;
    }
    i
}

fn join(lines: Vec<String>) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

fn is_bullet(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with("- ") || t.starts_with("* ")
}

fn strip_now(l: &str) -> String {
    let t = l.trim_end();
    match t.strip_suffix(NOW_TAG) {
        Some(rest) => rest.trim_end().to_string(),
        None => t.to_string(),
    }
}

fn parse_item(l: &str) -> Option<Item> {
    let t = l.trim_start();
    let body = t.strip_prefix("- ").or_else(|| t.strip_prefix("* "))?;
    let (done, rest) = if let Some(r) = body
        .strip_prefix("[x] ")
        .or_else(|| body.strip_prefix("[X] "))
    {
        (true, r)
    } else if let Some(r) = body.strip_prefix("[ ] ") {
        (false, r)
    } else {
        (false, body)
    };
    let now = rest.trim_end().ends_with(NOW_TAG);
    let text = strip_now(rest).trim().to_string();
    Some(Item { text, done, now })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 6).unwrap()
    }

    const NOTE: &str = "---\ncreated: x\n---\n# 2026-09-06\n\n## Focus\n- [ ] ship portal tests\n- [x] clock in\n- write roadmap\n\n## Tasks\n\n### Health\n- [ ] run\n- [x] stretch\n\n### Logistics\n- [ ] renew parking\n\n## Record\nWritten at the end of the day.\n- 09:00 stood up\n\n## Notes\n\n## Thoughts\nkeep this\n";

    #[test]
    fn reads_focus_and_task_counts() {
        let n = Daily::parse(d(), NOTE);
        let s = n.snapshot();
        assert_eq!(s.focus.len(), 3);
        assert_eq!(
            s.focus[2],
            Item {
                text: "write roadmap".into(),
                done: false,
                now: false
            }
        );
        assert_eq!(s.tasks_open, 2);
        assert_eq!(s.tasks_done, 1);
        assert_eq!(s.record_entries, 1);
        assert_eq!(s.now, None);
        assert_eq!(s.line(), "▶ no focus set · focus 1/3 · 2 open · 1 logged");
    }

    #[test]
    fn set_now_tags_the_matching_focus_line() {
        let mut n = Daily::parse(d(), NOTE);
        assert_eq!(n.set_now("portal"), "ship portal tests");
        assert!(n.raw.contains("- [ ] ship portal tests #now\n"));
        assert_eq!(n.snapshot().now.as_deref(), Some("ship portal tests"));
        assert_eq!(n.set_now("roadmap"), "write roadmap");
        assert!(!n.raw.contains("ship portal tests #now"));
        assert!(n.raw.contains("- write roadmap #now\n"));
    }

    #[test]
    fn set_now_adds_a_new_bullet_when_nothing_matches() {
        let mut n = Daily::parse(d(), NOTE);
        n.set_now("review PR #69");
        assert!(n
            .raw
            .contains("- write roadmap\n- [ ] review PR #69 #now\n\n## Tasks"));
        assert_eq!(n.focus_items().len(), 4);
    }

    #[test]
    fn mark_done_checks_focus_then_tasks_and_drops_now() {
        let mut n = Daily::parse(d(), NOTE);
        n.set_now("portal");
        assert_eq!(n.mark_done("portal").unwrap(), "ship portal tests");
        assert!(n.raw.contains("- [x] ship portal tests\n"));
        assert_eq!(n.snapshot().now, None);
        assert_eq!(n.mark_done("parking").unwrap(), "renew parking");
        assert!(n.raw.contains("- [x] renew parking"));
        assert!(matches!(
            n.mark_done("nothing here"),
            Err(VaultError::NoMatch(_))
        ));
    }

    #[test]
    fn append_keeps_the_blank_line_before_the_next_heading() {
        let mut n = Daily::parse(d(), NOTE);
        n.capture("idea: focus strip in tmux");
        assert!(n
            .raw
            .contains("## Notes\n- idea: focus strip in tmux\n\n## Thoughts\nkeep this\n"));
        n.append("Record", "10:15 wrote tests");
        assert!(n
            .raw
            .contains("- 09:00 stood up\n- 10:15 wrote tests\n\n## Notes"));
    }

    #[test]
    fn append_creates_a_missing_section_at_the_end() {
        let mut n = Daily::parse(d(), "# 2026-09-06\n\n## Focus\n- a\n");
        n.capture("x");
        assert_eq!(n.raw, "# 2026-09-06\n\n## Focus\n- a\n\n## Notes\n- x\n");
    }

    #[test]
    fn skeleton_has_the_four_sections() {
        let s = skeleton(d());
        for name in SECTIONS {
            assert!(s.contains(&format!("## {name}")));
        }
        let n = Daily::parse(d(), &s);
        assert_eq!(n.snapshot().line(), "▶ no focus set");
    }

    #[test]
    fn vault_maps_a_date_to_a_file_and_rejects_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::new(dir.path().to_str().unwrap(), "Daily Notes/{date}.md").unwrap();
        assert!(v.note_path(d()).ends_with("Daily Notes/2026-09-06.md"));
        assert!(matches!(Vault::new("", "x"), Err(VaultError::NoPath)));
        assert!(matches!(
            Vault::new("/definitely/not/here", "x"),
            Err(VaultError::Missing(_))
        ));
    }

    #[test]
    fn load_save_round_trip_creates_the_note() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::new(dir.path().to_str().unwrap(), "{date}.md").unwrap();
        let mut n = v.load(d()).unwrap();
        assert!(!n.snapshot().exists);
        n.set_now("first thing");
        n.save().unwrap();
        let again = v.load(d()).unwrap();
        assert!(again.snapshot().exists);
        assert_eq!(again.snapshot().now.as_deref(), Some("first thing"));
    }
}
