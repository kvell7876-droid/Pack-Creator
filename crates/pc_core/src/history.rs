//! Earlier versions of a file. Every save is autosave, so a mistake is saved a moment after it is made; this keeps copies of what a file was, in the project's own
//! `.packcreator/history` folder (which is never exported, and never shown in the tree), so the person can look at an old version and bring it back.
//!
//! A copy is kept when a file is saved with text that differs from the newest copy and at least `GAP_SECONDS` have passed since it. The first time a file is saved, the text it
//! had before is kept too, so what the file was when it was opened is always there. Only the newest `KEEP` copies are kept.

use crate::error::{Error, Result};
use crate::project::META_DIR;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const GAP_SECONDS: u64 = 30;
pub const KEEP: usize = 40;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Version {
    /// Milliseconds since 1970, which is also the copy's name.
    pub id: u64,
    pub size: u64,
}

fn folder(root: &Path, rel: &str) -> PathBuf {
    /* One folder for each file, named for its path with the slashes made safe. */
    root.join(META_DIR).join("history").join(rel.replace(['/', '\\', ':'], "~"))
}

fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn versions_in(dir: &Path) -> Vec<Version> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<Version> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let id = name.strip_suffix(".txt")?.parse().ok()?;

            Some(Version { id, size: e.metadata().map(|m| m.len()).unwrap_or(0) })
        })
        .collect();

    found.sort_by(|a, b| b.id.cmp(&a.id));
    found
}

/// The versions kept of a file, newest first.
pub fn list(root: &Path, rel: &str) -> Vec<Version> {
    versions_in(&folder(root, rel))
}

/// The text of one version.
pub fn read(root: &Path, rel: &str, id: u64) -> Result<String> {
    let path = folder(root, rel).join(format!("{id}.txt"));

    fs::read_to_string(&path).map_err(|e| Error::io(&path, e))
}

/// Called with a file's new text just before it is written, and what the file held until now (None for a new file). Never fails the save: history is a courtesy.
pub fn record(root: &Path, rel: &str, previous: Option<&str>, new_text: &str, now: u64) {
    let dir = folder(root, rel);
    let existing = versions_in(&dir);

    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let put = |id: u64, text: &str| {
        let _ = fs::write(dir.join(format!("{id}.txt")), text);
    };

    let Some(newest) = existing.first() else {
        /* The first save of this file: what it was is kept, then the new text. */
        if let Some(before) = previous.filter(|p| *p != new_text) {
            put(now.saturating_sub(1), before);
        }

        put(now, new_text);
        return;
    };

    if now.saturating_sub(newest.id) < GAP_SECONDS * 1000 {
        return;
    }

    if read(root, rel, newest.id).map_or(false, |text| text == new_text) {
        return;
    }

    put(now, new_text);

    for old in existing.iter().skip(KEEP - 1) {
        let _ = fs::remove_file(dir.join(format!("{}.txt", old.id)));
    }
}

/// `record` with the clock.
pub fn record_now(root: &Path, rel: &str, previous: Option<&str>, new_text: &str) {
    record(root, rel, previous, new_text, now_millis());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_text_and_then_a_copy_each_gap() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        record(root, "data/x/girls/a.json", Some("old"), "one", 1_000_000);
        assert_eq!(list(root, "data/x/girls/a.json").len(), 2, "what it was, and the first save");

        record(root, "data/x/girls/a.json", Some("one"), "two", 1_010_000);
        assert_eq!(list(root, "data/x/girls/a.json").len(), 2, "too soon after the last copy");

        record(root, "data/x/girls/a.json", Some("two"), "three", 1_040_000);

        let versions = list(root, "data/x/girls/a.json");

        assert_eq!(versions.len(), 3);
        assert_eq!(read(root, "data/x/girls/a.json", versions[0].id).unwrap(), "three");
        assert_eq!(read(root, "data/x/girls/a.json", versions[2].id).unwrap(), "old");

        /* The same text again is not a new copy. */
        record(root, "data/x/girls/a.json", Some("three"), "three", 1_100_000);
        assert_eq!(list(root, "data/x/girls/a.json").len(), 3);
    }

    #[test]
    fn only_the_newest_are_kept() {
        let dir = tempfile::tempdir().unwrap();

        for n in 0..(KEEP as u64 + 10) {
            record(dir.path(), "a.json", None, &format!("text {n}"), 1_000_000 + n * 60_000);
        }

        assert_eq!(list(dir.path(), "a.json").len(), KEEP);
    }
}
