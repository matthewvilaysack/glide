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
    #[error("{0}")]
    Io(String),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Entry {
    pub path: String,
    pub modified: String,
}

const MAX_HITS: usize = 50;
const MAX_LIST: usize = 200;

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

    pub fn search(&self, query: &str, folder: Option<&str>) -> Vec<Hit> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let prefix = folder
            .map(|f| f.trim_matches('/').to_string())
            .filter(|f| !f.is_empty());
        let mut hits = Vec::new();
        for (abs, rel) in self.files() {
            if let Some(p) = &prefix {
                if !rel.starts_with(&format!("{p}/")) {
                    continue;
                }
            }
            if rel.to_lowercase().contains(&needle) {
                hits.push(Hit {
                    path: rel.clone(),
                    line: 0,
                    text: "(title match)".into(),
                });
            }
            let Ok(body) = std::fs::read_to_string(&abs) else {
                continue;
            };
            for (i, line) in body.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    hits.push(Hit {
                        path: rel.clone(),
                        line: i + 1,
                        text: line.trim().chars().take(240).collect(),
                    });
                    if hits.len() >= MAX_HITS {
                        return hits;
                    }
                }
            }
        }
        hits
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
            .search("glide", None)
            .into_iter()
            .map(|h| h.path)
            .collect();
        assert!(paths.contains(&"Projects/Glide.md".to_string()));
        assert!(paths.contains(&"Ideas.md".to_string()));
        assert!(!paths
            .iter()
            .any(|p| p.starts_with("Personal") || p.starts_with(".obsidian")));
        assert!(n.search("secret", None).is_empty());
    }

    #[test]
    fn search_and_list_respect_a_folder_filter() {
        let (_d, n) = vault();
        assert!(n
            .search("glide", Some("Projects"))
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
}
