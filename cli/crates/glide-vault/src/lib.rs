//! The vault contract: one Markdown daily note per day, in a folder the user
//! already syncs (Obsidian over iCloud, or any folder sync). Glide reads and
//! edits four `##` sections of today's note and never touches anything else.
//!
//! - `## Focus`: the day's priorities as bullets; the one tagged `#now` is the
//!   current focus.
//! - `## Tasks`: checkbox bullets, sub-headings allowed.
//! - `## Record`: what actually happened, appended as `- HH:MM text`.
//! - `## Notes`: quick captures, appended as `- text`.

pub mod decide;
pub mod sprint;

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

/// The four headings glide touches. Configurable, because daily-note
/// templates differ: one person's Focus list is another's Checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sections {
    pub focus: String,
    pub tasks: String,
    pub record: String,
    pub notes: String,
}

impl Default for Sections {
    fn default() -> Self {
        Sections {
            focus: "Focus".into(),
            tasks: "Tasks".into(),
            record: "Record".into(),
            notes: "Notes".into(),
        }
    }
}

impl Sections {
    fn all(&self) -> [&str; 4] {
        [&self.focus, &self.tasks, &self.record, &self.notes]
    }
}

/// Where the notes live and how a day maps to a file.
#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
    /// Relative pattern with `{date}` for `YYYY-MM-DD`, e.g. `Daily Notes/{date}.md`.
    pub daily_note_pattern: String,
    pub sections: Sections,
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
            sections: Sections::default(),
        })
    }

    pub fn with_sections(mut self, sections: Sections) -> Self {
        self.sections = sections;
        self
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
        let exists = path.exists();
        let raw = if exists {
            std::fs::read_to_string(&path)?
        } else {
            skeleton(date, &self.sections)
        };
        Ok(Daily {
            path,
            date,
            raw,
            sections: self.sections.clone(),
            exists,
        })
    }
}

fn expand_home(p: &str) -> PathBuf {
    #[cfg(feature = "home")]
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(base) = directories::BaseDirs::new() {
            return base.home_dir().join(rest);
        }
    }
    PathBuf::from(p)
}

fn skeleton(date: NaiveDate, sections: &Sections) -> String {
    let mut s = format!("# {}\n", date.format("%Y-%m-%d"));
    for name in sections.all() {
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

/// The text an agent needs at the start of a session, and nothing else. Shared
/// by `glide prime` and the browser playground so both show the same words.
pub fn prime_text(snap: Option<&Snapshot>) -> String {
    prime_text_with(snap, &[])
}

/// The same, plus what has already been settled.
///
/// Decisions go LAST, immediately before the workflow instructions, because a fact
/// placed mid-context is likelier to be missed than one at either end and the
/// rejections are the half that saves turns. Capped, because the value is in the
/// agent reading them, and a list nobody finishes is a list nobody reads.
pub fn prime_text_with(snap: Option<&Snapshot>, decisions: &[&decide::Decision]) -> String {
    let mut s = String::new();
    s.push_str("## glide: this person's priorities\n\n");
    match snap {
        Some(snap) => {
            s.push_str(&format!("Today ({}): {}\n", snap.date, snap.line()));
            if let Some(now) = &snap.now {
                s.push_str(&format!("Current focus: {now}\n"));
            }
            if !snap.focus.is_empty() {
                s.push_str("Focus list:\n");
                for item in &snap.focus {
                    let mark = if item.done { "[x]" } else { "[ ]" };
                    let now = if item.now { " (now)" } else { "" };
                    s.push_str(&format!("- {mark} {}{now}\n", item.text));
                }
            }
            if !snap.exists {
                s.push_str("(No note for today yet; the first write creates it.)\n");
            }
        }
        None => s.push_str("Today: unknown, the vault is not set up.\n"),
    }
    if !decisions.is_empty() {
        s.push_str("\nAlready settled, do not propose otherwise without saying why:\n");
        for d in decisions {
            s.push_str(&format!("- {}\n", d.line()));
        }
    }
    s.push_str(
        "\nWorkflow: mention the current focus in one line at the start. When the person says what they are on, run `glide focus set <text>`. \
         When something finishes, `glide focus done <text>`. After every task you complete, `glide focus log <one or two sentences>`, without being asked. \
         Anything they say to remember: `glide focus capture <text>`. \
         When they settle a question, `glide decide <text>`, and when they rule something out, `glide decide --against <text>`. Read the whole note with `glide focus today`. Add `--json` for structured output. \
         Never edit the daily note by hand; these verbs are the only writers.\n",
    );
    s
}

/// A loaded daily note. Edits work on the raw text so everything outside the
/// four sections survives byte for byte.
#[derive(Debug, Clone)]
pub struct Daily {
    pub path: PathBuf,
    pub date: NaiveDate,
    pub raw: String,
    pub sections: Sections,
    /// Whether the note was read from disk (or handed over as text) rather
    /// than made up from the skeleton. Drives the "no note yet" hints.
    pub exists: bool,
}

impl Daily {
    pub fn parse(date: NaiveDate, raw: &str) -> Self {
        Daily {
            path: PathBuf::new(),
            date,
            raw: raw.to_string(),
            sections: Sections::default(),
            exists: true,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let focus = self.focus_items();
        let tasks = self.task_items();
        Snapshot {
            date: self.date.format("%Y-%m-%d").to_string(),
            path: self.path.display().to_string(),
            exists: self.exists,
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
        self.section_lines(&self.sections.focus.clone())
            .iter()
            .filter_map(|l| parse_item(l))
            .collect()
    }

    pub fn task_items(&self) -> Vec<Item> {
        self.section_lines(&self.sections.tasks.clone())
            .iter()
            .filter_map(|l| parse_item(l))
            .filter(|i| !i.text.is_empty())
            .collect()
    }

    /// Every open item in Focus and Tasks, in the order they appear.
    ///
    /// The preview `clear` prints before it is allowed to remove anything, and the
    /// same list the removal walks, so what a person is shown and what happens to
    /// their note cannot drift apart.
    pub fn open_items(&self) -> Vec<String> {
        let mut out = Vec::new();
        for section in [self.sections.focus.clone(), self.sections.tasks.clone()] {
            for line in self.section_lines(&section) {
                if let Some(item) = parse_item(&line) {
                    if !item.done && !item.text.is_empty() && is_checkbox(&line) {
                        out.push(item.text);
                    }
                }
            }
        }
        out
    }

    /// Drop the open items from Focus and Tasks, keeping everything else.
    ///
    /// Done bullets stay, because they are the day's record of what happened and the
    /// thing being cleared is what did not. A bullet with no checkbox stays too: it
    /// is prose someone wrote in a list rather than a task they left open, and
    /// deleting it would take writing nobody asked to remove.
    ///
    /// Returns what it removed, so the caller can say so rather than reporting a
    /// count the person has to trust.
    pub fn clear_open(&mut self) -> Vec<String> {
        let removed = self.open_items();
        if removed.is_empty() {
            return removed;
        }
        for section in [self.sections.focus.clone(), self.sections.tasks.clone()] {
            let (start, end) = self.section_span(&section);
            if start == end {
                continue;
            }
            let lines: Vec<String> = self.raw.lines().map(String::from).collect();
            let kept: Vec<String> = lines[start..end]
                .iter()
                .filter(|l| match parse_item(l) {
                    Some(item) => item.done || item.text.is_empty() || !is_checkbox(l),
                    None => true,
                })
                .cloned()
                .collect();
            let mut out = lines[..start].to_vec();
            out.extend(kept);
            out.extend_from_slice(&lines[end..]);
            self.raw = format!("{}\n", out.join("\n"));
        }
        removed
    }

    /// Make `text` the current focus. Matches an existing Focus bullet
    /// case-insensitively by substring, else adds a new one. Returns the text
    /// of the bullet that now carries the tag.
    pub fn set_now(&mut self, text: &str) -> String {
        let focus = self.sections.focus.clone();
        self.ensure_section(&focus);
        let (start, end) = self.section_span(&focus);
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
        for name in [self.sections.focus.clone(), self.sections.tasks.clone()] {
            let (start, end) = self.section_span(&name);
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

    /// Add `## section` at the end of the note if it is missing, so a note made
    /// by some other template (one without a Focus heading, say) still takes
    /// the verbs without a bullet landing on line one.
    fn ensure_section(&mut self, section: &str) {
        let lines: Vec<String> = self.raw.lines().map(String::from).collect();
        if self.find_section(&lines, section).is_some() {
            return;
        }
        let mut out = self.raw.trim_end_matches('\n').to_string();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("## {section}\n"));
        self.raw = out;
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
        let notes = self.sections.notes.clone();
        self.append(&notes, text);
    }

    /// Append `- HH:MM text` to Record using the given clock reading. The
    /// browser build has no clock and calls this directly.
    pub fn log_at(&mut self, stamp: &str, text: &str) {
        let record = self.sections.record.clone();
        self.append(&record, &format!("{stamp} {}", text.trim()));
    }

    pub fn log(&mut self, text: &str) {
        let stamp = Local::now().format("%H:%M").to_string();
        self.log_at(&stamp, text);
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.raw)?;
        self.exists = true;
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

/// Whether a bullet carries a checkbox, which is what separates a task from prose.
fn is_checkbox(l: &str) -> bool {
    let t = l.trim_start();
    let body = t.strip_prefix("- ").or_else(|| t.strip_prefix("* "));
    match body {
        Some(b) => b.starts_with("[ ] ") || b.starts_with("[x] ") || b.starts_with("[X] "),
        None => false,
    }
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
    fn clear_removes_unchecked_and_keeps_done() {
        // The day's record is what was finished. Clearing is for the things that did
        // not happen, so the done bullets stay and only the open ones go.
        let mut n = Daily::parse(d(), NOTE);
        let removed = n.clear_open();
        assert_eq!(removed.len(), 3, "two open tasks and one open focus item");
        assert!(n.raw.contains("- [x] clock in"));
        assert!(n.raw.contains("- [x] stretch"));
        assert!(!n.raw.contains("ship portal tests"));
        assert!(!n.raw.contains("renew parking"));
    }

    #[test]
    fn clear_leaves_a_bullet_that_is_not_a_checkbox_alone() {
        // "- write roadmap" carries no checkbox, so it is prose in a list rather than
        // a task anyone ticked. Deleting it would lose writing nobody marked open.
        let mut n = Daily::parse(d(), NOTE);
        n.clear_open();
        assert!(n.raw.contains("- write roadmap"));
    }

    #[test]
    fn clear_touches_no_section_but_focus_and_tasks() {
        let mut n = Daily::parse(d(), NOTE);
        n.clear_open();
        assert!(n.raw.contains("- 09:00 stood up"), "Record is untouched");
        assert!(n.raw.contains("keep this"), "Thoughts is untouched");
    }

    #[test]
    fn clear_reports_what_it_removed_before_removing_it() {
        let mut n = Daily::parse(d(), NOTE);
        let preview = n.open_items();
        let removed = n.clear_open();
        assert_eq!(preview, removed, "the dry run and the real run agree");
    }

    #[test]
    fn clear_on_an_already_clear_day_removes_nothing() {
        let mut n = Daily::parse(d(), "# x\n\n## Focus\n- [x] done\n\n## Tasks\n");
        assert!(n.clear_open().is_empty());
    }

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
    fn set_now_creates_focus_when_a_template_note_lacks_it() {
        let mut n = Daily::parse(
            d(),
            "---\ncreated: x\n---\n# 2026-09-07\n\n## Baseline\n- [ ] run\n",
        );
        assert_eq!(n.set_now("write the roadmap"), "write the roadmap");
        assert!(
            n.raw.starts_with("---\ncreated: x\n---\n"),
            "frontmatter must stay first"
        );
        assert!(n
            .raw
            .ends_with("## Baseline\n- [ ] run\n\n## Focus\n- [ ] write the roadmap #now\n"));
        assert_eq!(n.snapshot().now.as_deref(), Some("write the roadmap"));
        assert_eq!(n.mark_done("roadmap").unwrap(), "write the roadmap");
    }

    #[test]
    fn configured_headings_are_used_everywhere() {
        let mut n = Daily::parse(
            d(),
            "# day\n\n## Checklist\n- [ ] apply to a16z\n- [ ] email Hari\n\n## Record\n",
        );
        n.sections = Sections {
            focus: "Checklist".into(),
            ..Sections::default()
        };
        assert_eq!(n.snapshot().focus.len(), 2);
        assert_eq!(n.set_now("a16z"), "apply to a16z");
        assert!(n.raw.contains("- [ ] apply to a16z #now\n- [ ] email Hari"));
        assert_eq!(n.mark_done("hari").unwrap(), "email Hari");
        assert!(!n.raw.contains("## Focus"));
    }

    #[test]
    fn skeleton_has_the_four_sections() {
        let s = skeleton(d(), &Sections::default());
        for name in Sections::default().all() {
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
        assert!(prime_text(Some(&n.snapshot())).contains("No note for today yet"));
        n.set_now("first thing");
        n.save().unwrap();
        assert!(n.snapshot().exists);
        let again = v.load(d()).unwrap();
        assert!(again.snapshot().exists);
        assert_eq!(again.snapshot().now.as_deref(), Some("first thing"));
    }

    #[test]
    fn a_parsed_note_reports_as_existing() {
        let d = Daily::parse(
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
            "# 2026-09-07\n\n## Focus\n",
        );
        assert!(d.snapshot().exists);
    }

    #[test]
    fn log_at_uses_the_given_stamp() {
        let mut d = Daily::parse(
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
            "# 2026-09-07\n\n## Record\n\n## Notes\n",
        );
        d.log_at("10:42", "fixed the flaky build");
        assert!(d.raw.contains("- 10:42 fixed the flaky build\n"));
        assert_eq!(d.snapshot().record_entries, 1);
    }

    #[test]
    fn prime_text_names_the_current_focus_and_the_workflow() {
        let mut d = Daily::parse(
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
            "# 2026-09-07\n\n## Focus\n- [ ] write the roadmap\n\n## Tasks\n\n## Record\n\n## Notes\n",
        );
        d.set_now("roadmap");
        let t = prime_text(Some(&d.snapshot()));
        assert!(t.starts_with("## glide: this person's priorities\n\nToday (2026-09-07): "));
        assert!(t.contains("Current focus: write the roadmap\n"));
        assert!(t.contains("- [ ] write the roadmap (now)\n"));
        assert!(!t.contains("No note for today yet"));
        assert!(t.contains("Workflow: mention the current focus"));
        assert!(prime_text(None).contains("Today: unknown, the vault is not set up."));
    }
}

#[cfg(test)]
mod prime_decision_tests {
    use super::*;

    #[test]
    fn prime_carries_what_was_ruled_out() {
        // The whole point. An agent that does not know a thing was rejected proposes
        // it again, and the cost of that is turns, not the sentence.
        let d = decide::parse("- 2026-09-02 ruled out: mongo, schema churn bit us\n");
        let refs: Vec<&decide::Decision> = d.iter().collect();
        let text = prime_text_with(None, &refs);
        assert!(text.contains("ruled out: mongo"));
        assert!(text.contains("do not propose otherwise"));
    }

    #[test]
    fn decisions_sit_after_the_focus_and_before_the_workflow() {
        let d = decide::parse("- 2026-09-02 decided: postgres\n");
        let refs: Vec<&decide::Decision> = d.iter().collect();
        let t = prime_text_with(None, &refs);
        assert!(t.find("decided: postgres").unwrap() < t.find("Workflow:").unwrap());
    }

    #[test]
    fn nothing_is_added_when_nothing_is_settled() {
        assert!(!prime_text_with(None, &[]).contains("Already settled"));
    }
}
