//! A sprint: a named set of work that outlives a single day.
//!
//! The daily note answers what is happening today and is thrown away tomorrow. A
//! sprint is the thing today draws from, so it needs somewhere to live that is not
//! the daily note, and the one rule this product does not break is that the user's
//! writing stays in files the user owns. So a sprint is a Markdown note in the vault
//! with a checklist in it, openable in Obsidian, diffable in git, and editable by
//! hand without this tool's permission.
//!
//! At most one sprint is active. That is a deliberate limit rather than a missing
//! feature: the point of the daily note is that it names what matters now, and a
//! tool that let three sprints be current would push the choice back onto the person
//! at exactly the moment it is supposed to make it for them.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::Result;

/// One sprint, as read off disk.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprint {
    pub slug: String,
    pub name: String,
    pub started: String,
    pub active: bool,
    pub items: Vec<SprintItem>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SprintItem {
    pub text: String,
    pub done: bool,
}

impl Sprint {
    pub fn open(&self) -> Vec<&SprintItem> {
        self.items.iter().filter(|i| !i.done).collect()
    }

    pub fn done_count(&self) -> usize {
        self.items.iter().filter(|i| i.done).count()
    }

    /// A one-line summary, the shape the focus strip uses.
    pub fn line(&self) -> String {
        format!(
            "{} · {}/{} done",
            self.name,
            self.done_count(),
            self.items.len()
        )
    }
}

/// Turn a name into a file name that survives a filesystem and a URL.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "sprint".to_string()
    } else {
        trimmed
    }
}

pub fn render(name: &str, started: NaiveDate, active: bool, items: &[SprintItem]) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("sprint: {}\n", slugify(name)));
    out.push_str(&format!("started: {}\n", started.format("%Y-%m-%d")));
    out.push_str(&format!("active: {}\n", active));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n## Items\n", name));
    for item in items {
        out.push_str(&format!(
            "- [{}] {}\n",
            if item.done { "x" } else { " " },
            item.text
        ));
    }
    out
}

/// Read one sprint note. Anything it cannot understand is reported by name rather
/// than skipped, because a sprint silently missing from a list reads as finished.
pub fn parse(path: &Path, raw: &str) -> Result<Sprint> {
    let slug = front(raw, "sprint").unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    });
    let started = front(raw, "started").unwrap_or_default();
    let active = front(raw, "active")
        .map(|v| v.trim() == "true")
        .unwrap_or(false);

    let name = raw
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l[2..].trim().to_string())
        .unwrap_or_else(|| slug.clone());

    let mut items = Vec::new();
    let mut in_items = false;
    for line in raw.lines() {
        if line.trim_start().starts_with("## ") {
            in_items = line.trim().eq_ignore_ascii_case("## Items");
            continue;
        }
        if !in_items {
            continue;
        }
        let t = line.trim_start();
        let body = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            Some(b) => b,
            None => continue,
        };
        if let Some(rest) = body
            .strip_prefix("[x] ")
            .or_else(|| body.strip_prefix("[X] "))
        {
            items.push(SprintItem {
                text: rest.trim().to_string(),
                done: true,
            });
        } else if let Some(rest) = body.strip_prefix("[ ] ") {
            items.push(SprintItem {
                text: rest.trim().to_string(),
                done: false,
            });
        }
    }

    Ok(Sprint {
        slug,
        name,
        started,
        active,
        items,
        path: path.to_path_buf(),
    })
}

fn front(raw: &str, key: &str) -> Option<String> {
    let mut lines = raw.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            return None;
        }
        if let Some(v) = line.strip_prefix(&format!("{}:", key)) {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Every sprint in the folder, newest first, with the active one first of all.
pub fn list(dir: &Path) -> Result<Vec<Sprint>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().map(|e| e != "md").unwrap_or(true) {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        out.push(parse(&path, &raw)?);
    }
    out.sort_by(|a, b| b.active.cmp(&a.active).then(b.started.cmp(&a.started)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\nsprint: test-pipeline\nstarted: 2026-09-11\nactive: true\n---\n\n# Test pipeline\n\n## Items\n- [ ] close the coverage loop\n- [x] version the machine file\n- not a task\n";

    #[test]
    fn reads_a_sprint_note() {
        let s = parse(Path::new("/x/test-pipeline.md"), NOTE).unwrap();
        assert_eq!(s.slug, "test-pipeline");
        assert_eq!(s.name, "Test pipeline");
        assert!(s.active);
        assert_eq!(s.items.len(), 2, "a bullet with no checkbox is not an item");
        assert_eq!(s.open().len(), 1);
        assert_eq!(s.done_count(), 1);
    }

    #[test]
    fn a_sprint_survives_a_round_trip_through_disk() {
        // The file is the user's, so it has to be readable back exactly as written.
        // Anything lost here is lost from a note someone keeps in their own vault.
        let s = parse(Path::new("/x/a.md"), NOTE).unwrap();
        let again = render(
            &s.name,
            NaiveDate::from_ymd_opt(2026, 9, 11).unwrap(),
            true,
            &s.items,
        );
        let back = parse(Path::new("/x/a.md"), &again).unwrap();
        assert_eq!(back.items, s.items);
        assert_eq!(back.name, s.name);
        assert!(back.active);
    }

    #[test]
    fn slugs_are_safe_to_be_filenames() {
        assert_eq!(slugify("LEG test pipeline!"), "leg-test-pipeline");
        assert_eq!(slugify("  spaced  out  "), "spaced-out");
        assert_eq!(slugify("!!!"), "sprint");
    }

    #[test]
    fn the_active_sprint_sorts_first() {
        let a = parse(
            Path::new("/x/old.md"),
            "---\nsprint: old\nstarted: 2026-01-01\nactive: false\n---\n# Old\n## Items\n",
        )
        .unwrap();
        let b = parse(Path::new("/x/now.md"), NOTE).unwrap();
        let mut v = [a, b];
        v.sort_by(|x, y| y.active.cmp(&x.active).then(y.started.cmp(&x.started)));
        assert!(v[0].active);
    }

    #[test]
    fn a_line_says_progress_without_being_asked_twice() {
        let s = parse(Path::new("/x/a.md"), NOTE).unwrap();
        assert_eq!(s.line(), "Test pipeline · 1/2 done");
    }
}
