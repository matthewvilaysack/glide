//! Read access to the whole vault for remote clients. Every path goes through
//! `Notes::resolve`, which is the one place that decides what a client may see:
//! Markdown only, inside the vault root, outside hidden and excluded folders.
//! A refused path reads as "not found" so an excluded folder does not leak
//! its own existence.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Notes {
    root: PathBuf,
    exclude: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum NotesError {
    #[error("note not found: {0}")]
    NotFound(String),
    #[error("{0} matches more than one note; use one of these paths: {1}")]
    Ambiguous(String, String),
    #[error("{0}")]
    Io(String),
}

/// One note that matched a search, with the lines that matched best.
#[derive(Debug, Serialize, PartialEq)]
pub struct NoteHit {
    pub path: String,
    pub matches: Vec<LineHit>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct LineHit {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Entry {
    pub path: String,
    pub modified: String,
}

const MAX_LIST: usize = 200;
const DEFAULT_RESULTS: usize = 10;
const MAX_RESULTS: usize = 25;
const LINES_PER_NOTE: usize = 3;

/// Words too common to tell notes apart. Dropped from the query unless the
/// query is nothing but these.
const STOPWORDS: &[&str] = &[
    "a", "about", "an", "and", "are", "as", "at", "be", "by", "did", "do", "does", "for", "from",
    "how", "i", "in", "is", "it", "me", "my", "of", "on", "or", "the", "to", "was", "what", "when",
    "where", "which", "who", "why", "with",
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn query_terms(query: &str) -> Vec<String> {
    let all = words(query);
    let kept: Vec<String> = all
        .iter()
        .filter(|w| !STOPWORDS.contains(&w.as_str()))
        .cloned()
        .collect();
    let mut terms = if kept.is_empty() { all } else { kept };
    terms.dedup();
    terms
}

fn stem_of(rel: &str) -> String {
    rel.rsplit('/')
        .next()
        .unwrap_or(rel)
        .trim_end_matches(".md")
        .to_lowercase()
}

/// "interview" finds "Interviews", but "thai" does not find "Thailand".
fn title_has(title: &[String], term: &str) -> bool {
    title
        .iter()
        .any(|w| w == term || (term.len() >= 5 && w.starts_with(term)))
}

/// How well one note answers the query, or `None` when it matches too few terms.
fn score_note(
    rel: &str,
    body: &str,
    terms: &[String],
    phrase: &str,
) -> Option<(f64, Vec<LineHit>)> {
    let title = words(&stem_of(rel));
    let folders = words(rel.rsplit_once('/').map(|(f, _)| f).unwrap_or(""));
    let lower = body.to_lowercase();
    let mut score = 0.0;
    let mut matched = 0usize;
    for t in terms {
        let in_title = title_has(&title, t);
        let in_body = lower.matches(t.as_str()).count();
        if in_title {
            score += 8.0;
        } else if folders.iter().any(|w| w == t) {
            score += 2.0;
        }
        score += (in_body.min(5)) as f64;
        if in_title || in_body > 0 || folders.iter().any(|w| w == t) {
            matched += 1;
        }
    }
    let n = terms.len();
    if matched == 0 || matched * 2 < n {
        return None;
    }
    if matched == n && !title.is_empty() && terms.iter().all(|t| title_has(&title, t)) {
        score += 10.0;
    }
    if n > 1 && (stem_of(rel).contains(phrase) || lower.contains(phrase)) {
        score += 6.0;
    }
    if title == terms {
        score += 6.0;
    }
    // A note that answers part of the question ranks below one that answers all of it.
    score *= matched as f64 / n as f64;

    let mut lines: Vec<(usize, usize, bool, LineHit)> = body
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let l = line.to_lowercase();
            let hits = terms.iter().filter(|t| l.contains(t.as_str())).count();
            (hits > 0).then(|| {
                let heading = line.trim_start().starts_with('#');
                (
                    hits,
                    i,
                    heading,
                    LineHit {
                        line: i + 1,
                        text: line.trim().chars().take(240).collect(),
                    },
                )
            })
        })
        .collect();
    let headings = lines.iter().filter(|(_, _, h, _)| *h).count();
    score += (headings.min(3) * 2) as f64;
    lines.sort_by(|a, b| b.0.cmp(&a.0).then(b.2.cmp(&a.2)).then(a.1.cmp(&b.1)));
    let best = lines
        .into_iter()
        .take(LINES_PER_NOTE)
        .map(|(_, _, _, h)| h)
        .collect();
    Some((score, best))
}

impl Notes {
    pub fn new(root: &Path, exclude: &[String]) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        let exclude = exclude
            .iter()
            .map(|e| PathBuf::from(e.trim_matches('/')))
            .filter(|e| !e.as_os_str().is_empty())
            .collect();
        Ok(Self { root, exclude })
    }

    /// Vault-relative path of a file, if a client is allowed to see it.
    fn visible(&self, abs: &Path) -> Option<String> {
        let rel = abs.strip_prefix(&self.root).ok()?;
        if rel.extension().and_then(|e| e.to_str()) != Some("md") {
            return None;
        }
        for c in rel.components() {
            match c {
                Component::Normal(s) if s.to_string_lossy().starts_with('.') => return None,
                Component::Normal(_) => {}
                _ => return None,
            }
        }
        if self.exclude.iter().any(|e| rel.starts_with(e)) {
            return None;
        }
        Some(rel.to_string_lossy().replace('\\', "/"))
    }

    /// Turn a client-supplied path into an absolute file the client may read.
    pub fn resolve(&self, rel: &str) -> Result<PathBuf, NotesError> {
        match self.resolve_path(rel) {
            Err(NotesError::NotFound(_)) if !rel.contains('/') => self.resolve_name(rel),
            other => other,
        }
    }

    /// Find a note by name alone, the way an Obsidian `[[link]]` does.
    fn resolve_name(&self, name: &str) -> Result<PathBuf, NotesError> {
        let want = name.trim().trim_end_matches(".md").to_lowercase();
        let found: Vec<(PathBuf, String)> = self
            .files()
            .filter(|(_, rel)| stem_of(rel) == want)
            .collect();
        match found.len() {
            0 => Err(NotesError::NotFound(name.to_string())),
            1 => Ok(found.into_iter().next().unwrap().0),
            _ => {
                let paths: Vec<String> = found.into_iter().map(|(_, r)| r).collect();
                Err(NotesError::Ambiguous(name.to_string(), paths.join(", ")))
            }
        }
    }

    fn resolve_path(&self, rel: &str) -> Result<PathBuf, NotesError> {
        let missing = || NotesError::NotFound(rel.to_string());
        let rel = rel.trim().trim_start_matches('/');
        let rel = if rel.ends_with(".md") {
            rel.to_string()
        } else {
            format!("{rel}.md")
        };
        let candidate = self.root.join(&rel);
        // Canonicalize so `..` and symlinks are judged by where they really point.
        let real = candidate.canonicalize().map_err(|_| missing())?;
        if !real.is_file() {
            return Err(missing());
        }
        self.visible(&real).map(|_| real).ok_or_else(missing)
    }

    pub fn read(&self, rel: &str) -> Result<String, NotesError> {
        let p = self.resolve(rel)?;
        std::fs::read_to_string(p).map_err(|e| NotesError::Io(e.to_string()))
    }

    fn files(&self) -> impl Iterator<Item = (PathBuf, String)> + '_ {
        WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter_map(move |e| {
                let p = e.into_path();
                self.visible(&p).map(|rel| (p, rel))
            })
    }

    /// Notes that best answer `query`, best first. Words are matched
    /// separately, so "heygen prep plan" finds "HeyGen Technical Interview Prep".
    pub fn search(&self, query: &str, folder: Option<&str>, limit: Option<usize>) -> Vec<NoteHit> {
        let terms = query_terms(query);
        if terms.is_empty() {
            return Vec::new();
        }
        let phrase = words(query).join(" ");
        let limit = limit.unwrap_or(DEFAULT_RESULTS).clamp(1, MAX_RESULTS);
        let prefix = folder
            .map(|f| f.trim_matches('/').to_string())
            .filter(|f| !f.is_empty());
        let mut scored: Vec<(f64, NoteHit)> = Vec::new();
        for (abs, rel) in self.files() {
            if let Some(p) = &prefix {
                if !rel.starts_with(&format!("{p}/")) {
                    continue;
                }
            }
            let Ok(body) = std::fs::read_to_string(&abs) else {
                continue;
            };
            if let Some((score, matches)) = score_note(&rel, &body, &terms, &phrase) {
                scored.push((score, NoteHit { path: rel, matches }));
            }
        }
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
        scored.into_iter().take(limit).map(|(_, h)| h).collect()
    }

    pub fn list(&self, folder: Option<&str>) -> Vec<Entry> {
        let prefix = folder
            .map(|f| f.trim_matches('/').to_string())
            .filter(|f| !f.is_empty());
        let mut out: Vec<(std::time::SystemTime, Entry)> = self
            .files()
            .filter(|(_, rel)| {
                prefix
                    .as_ref()
                    .is_none_or(|p| rel.starts_with(&format!("{p}/")))
            })
            .map(|(abs, rel)| {
                let t = std::fs::metadata(&abs)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                let modified = chrono::DateTime::<chrono::Local>::from(t)
                    .format("%Y-%m-%d %H:%M")
                    .to_string();
                (
                    t,
                    Entry {
                        path: rel,
                        modified,
                    },
                )
            })
            .collect();
        out.sort_by_key(|e| std::cmp::Reverse(e.0));
        out.into_iter().take(MAX_LIST).map(|(_, e)| e).collect()
    }

    /// Notes that link to `rel` with `[[name]]`, `[[name|alias]]` or `[[folder/name]]`.
    pub fn backlinks(&self, rel: &str) -> Result<Vec<String>, NotesError> {
        let target = self.resolve(rel)?;
        let target_rel = self
            .visible(&target)
            .ok_or_else(|| NotesError::NotFound(rel.into()))?;
        let full = target_rel.trim_end_matches(".md").to_lowercase();
        let stem = full.rsplit('/').next().unwrap_or(&full).to_string();
        let mut out = Vec::new();
        for (abs, other) in self.files() {
            if other == target_rel {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&abs) else {
                continue;
            };
            let lower = body.to_lowercase();
            let links = lower
                .split("[[")
                .skip(1)
                .filter_map(|s| s.split("]]").next());
            if links
                .map(|l| l.split(['|', '#']).next().unwrap_or("").trim())
                .any(|l| l == stem || l == full)
            {
                out.push(other);
            }
        }
        out.sort();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> (tempfile::TempDir, Notes) {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        std::fs::create_dir_all(r.join("Projects")).unwrap();
        std::fs::create_dir_all(r.join("Personal")).unwrap();
        std::fs::create_dir_all(r.join(".obsidian")).unwrap();
        std::fs::write(
            r.join("Projects/Glide.md"),
            "glide ships focus\nsee [[Ideas]]",
        )
        .unwrap();
        std::fs::write(r.join("Ideas.md"), "an idea about glide").unwrap();
        std::fs::write(r.join("Personal/Journal.md"), "secret glide thoughts").unwrap();
        std::fs::write(r.join(".obsidian/app.md"), "glide config").unwrap();
        std::fs::write(r.join("image.png"), "glide").unwrap();
        let n = Notes::new(r, &["Personal".into()]).unwrap();
        (d, n)
    }

    #[test]
    fn reads_a_visible_note_with_or_without_extension() {
        let (_d, n) = vault();
        assert_eq!(n.read("Ideas").unwrap(), "an idea about glide");
        assert_eq!(
            n.read("/Projects/Glide.md").unwrap(),
            "glide ships focus\nsee [[Ideas]]"
        );
    }

    #[test]
    fn refuses_paths_outside_the_vault() {
        let (d, n) = vault();
        let outside = d.path().parent().unwrap().join("outside.md");
        std::fs::write(&outside, "x").unwrap();
        assert!(matches!(
            n.read("../outside.md"),
            Err(NotesError::NotFound(_))
        ));
        assert!(matches!(
            n.read(outside.to_str().unwrap()),
            Err(NotesError::NotFound(_))
        ));
        std::fs::remove_file(outside).unwrap();
    }

    #[test]
    fn refuses_symlinks_that_leave_the_vault() {
        let (d, n) = vault();
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("x.md"), "outside").unwrap();
        std::os::unix::fs::symlink(other.path().join("x.md"), d.path().join("link.md")).unwrap();
        assert!(matches!(n.read("link.md"), Err(NotesError::NotFound(_))));
    }

    #[test]
    fn hidden_and_excluded_folders_and_non_markdown_read_as_not_found() {
        let (_d, n) = vault();
        assert!(matches!(
            n.read("Personal/Journal"),
            Err(NotesError::NotFound(_))
        ));
        assert!(matches!(
            n.read(".obsidian/app"),
            Err(NotesError::NotFound(_))
        ));
        assert!(matches!(n.read("image.png"), Err(NotesError::NotFound(_))));
    }

    #[test]
    fn search_skips_excluded_and_hidden_folders() {
        let (_d, n) = vault();
        let paths: Vec<_> = n
            .search("glide", None, None)
            .into_iter()
            .map(|h| h.path)
            .collect();
        assert!(paths.contains(&"Projects/Glide.md".to_string()));
        assert!(paths.contains(&"Ideas.md".to_string()));
        assert!(!paths
            .iter()
            .any(|p| p.starts_with("Personal") || p.starts_with(".obsidian")));
        assert!(n.search("secret", None, None).is_empty());
    }

    #[test]
    fn search_and_list_respect_a_folder_filter() {
        let (_d, n) = vault();
        assert!(n
            .search("glide", Some("Projects"), None)
            .iter()
            .all(|h| h.path.starts_with("Projects/")));
        let listed: Vec<_> = n.list(None).into_iter().map(|e| e.path).collect();
        assert_eq!(listed.len(), 2);
        assert!(n.list(Some("Personal")).is_empty());
    }

    #[test]
    fn backlinks_find_wikilinks_by_note_name() {
        let (_d, n) = vault();
        assert_eq!(
            n.backlinks("Ideas").unwrap(),
            vec!["Projects/Glide.md".to_string()]
        );
        assert!(n.backlinks("Personal/Journal").is_err());
    }

    fn study_vault() -> (tempfile::TempDir, Notes) {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for dir in ["Recruiting", "Archive", "Personal"] {
            std::fs::create_dir_all(r.join(dir)).unwrap();
        }
        std::fs::write(
            r.join("Recruiting/HeyGen Technical Interview Prep.md"),
            "# Plan\nday by day sprint for the HeyGen round\n## Prep\nAI-assisted Python",
        )
        .unwrap();
        std::fs::write(
            r.join("Recruiting/Questions to ask.md"),
            "- what problems do you face\n- For recruiters: ask what the hiring manager needs",
        )
        .unwrap();
        std::fs::write(
            r.join("Archive/Week 3.md"),
            "muay thai\nmuay thai\nmuay thai\nmuay thai",
        )
        .unwrap();
        std::fs::write(r.join("Muay Thai.md"), "# Muay Thai\nkick drills").unwrap();
        std::fs::write(r.join("Archive/Plan.md"), "old plan").unwrap();
        std::fs::write(
            r.join("Personal/HeyGen secret.md"),
            "heygen prep plan, private",
        )
        .unwrap();
        let n = Notes::new(r, &["Personal".into()]).unwrap();
        (d, n)
    }

    fn top(n: &Notes, q: &str) -> Vec<String> {
        n.search(q, None, None)
            .into_iter()
            .map(|h| h.path)
            .collect()
    }

    #[test]
    fn words_match_separately_so_a_loose_question_finds_the_note() {
        let (_d, n) = study_vault();
        assert_eq!(
            top(&n, "heygen prep plan")[0],
            "Recruiting/HeyGen Technical Interview Prep.md"
        );
        assert_eq!(
            top(&n, "questions to ask recruiters")[0],
            "Recruiting/Questions to ask.md"
        );
    }

    #[test]
    fn a_note_named_for_the_query_outranks_one_that_merely_repeats_it() {
        let (_d, n) = study_vault();
        assert_eq!(top(&n, "Muay Thai")[0], "Muay Thai.md");
    }

    #[test]
    fn each_result_carries_its_best_lines_and_the_limit_holds() {
        let (_d, n) = study_vault();
        let hits = n.search("heygen prep", None, Some(1));
        assert_eq!(hits.len(), 1);
        assert!(!hits[0].matches.is_empty() && hits[0].matches.len() <= LINES_PER_NOTE);
        assert!(hits[0].matches[0].line >= 1);
    }

    #[test]
    fn ranked_search_still_hides_excluded_folders() {
        let (_d, n) = study_vault();
        assert!(!top(&n, "heygen prep plan private")
            .iter()
            .any(|p| p.starts_with("Personal")));
    }

    #[test]
    fn notes_open_by_name_alone_like_an_obsidian_link() {
        let (_d, n) = study_vault();
        assert!(n
            .read("HeyGen Technical Interview Prep")
            .unwrap()
            .contains("sprint"));
        assert!(n.read("muay thai").unwrap().contains("kick drills"));
    }

    #[test]
    fn a_name_two_notes_share_asks_for_a_path_and_hidden_names_stay_hidden() {
        let (d, n) = study_vault();
        std::fs::write(d.path().join("Recruiting/Plan.md"), "new plan").unwrap();
        assert!(matches!(n.read("Plan"), Err(NotesError::Ambiguous(..))));
        assert!(matches!(
            n.read("HeyGen secret"),
            Err(NotesError::NotFound(_))
        ));
    }
}
