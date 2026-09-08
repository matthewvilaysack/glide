//! The browser engine and the binary must say the same thing for the demo
//! tape's commands. The binary runs against a temp vault seeded with the same
//! note; only the date and the Record clock stamps are allowed to differ.

use std::path::Path;
use std::process::Command;

use glide_play::{Session, LOG_STAMP, SEED_DATE, SEED_NOTE};

const TAPE: &[&str] = &[
    "glide focus",
    "glide focus set ship the portal tests",
    "glide focus set write the roadmap",
    "glide focus set portal",
    "glide focus log fixed the flaky build, retry on the artifact step",
    "glide focus capture ask about the cache revert",
    "glide focus done portal",
    "glide focus today",
    "glide prime",
];

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn binary(vault: &Path, line: &str) -> String {
    let words: Vec<&str> = line.split(' ').skip(1).collect();
    let out = Command::new(env!("CARGO_BIN_EXE_glide"))
        .arg("--no-color")
        .args(&words)
        .env_clear()
        .env("GLIDE_VAULT_PATH", vault)
        .env("HOME", vault)
        .current_dir(vault)
        .output()
        .expect("glide runs");
    assert!(
        out.status.success(),
        "{line}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8")
}

/// Fold the two things that legitimately differ: the real date and the real
/// clock.
fn normalise(s: &str, today: &str) -> String {
    let mut out = s.replace(today, SEED_DATE);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("- ") {
            if rest.len() > 6 && rest.as_bytes()[2] == b':' && rest.as_bytes()[5] == b' ' {
                out = out.replace(line, &format!("- {LOG_STAMP} {}", &rest[6..]));
            }
        }
    }
    out
}

#[test]
fn the_browser_engine_matches_the_binary_on_the_demo_tape() {
    // Read once; a run straddling local midnight can still differ because the
    // binary reads its own clock.
    let today = today();
    let dir = tempfile::tempdir().unwrap();
    let notes = dir.path().join("Daily Notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(
        notes.join(format!("{today}.md")),
        SEED_NOTE.replacen(SEED_DATE, &today, 1),
    )
    .unwrap();

    let mut play = Session::new();
    for line in TAPE {
        let argv: Vec<String> = line.split(' ').map(String::from).collect();
        let ours = play.run(argv);
        let theirs = binary(dir.path(), line);
        assert_eq!(
            normalise(&ours.stdout, &today),
            normalise(&theirs, &today),
            "stdout differs for `{line}`"
        );
    }

    let on_disk = std::fs::read_to_string(notes.join(format!("{today}.md"))).unwrap();
    assert_eq!(
        normalise(&play.note(), &today),
        normalise(&on_disk, &today),
        "final note differs"
    );
}
