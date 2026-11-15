//! The projects the home page lists: where they are and when each was last opened. Kept in one small file in the tool's own data folder.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recent {
    pub path: String,
    pub name: String,
    pub last_opened: u64,
}

/// A recent project as the home page shows it: whether its folder is still there is looked up each time.
#[derive(Debug, Clone, Serialize)]
pub struct RecentView {
    #[serde(flatten)]
    pub recent: Recent,
    pub exists: bool,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load(file: &Path) -> Vec<Recent> {
    fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn store(file: &Path, list: &[Recent]) -> Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    fs::write(file, serde_json::to_string_pretty(list).unwrap()).map_err(|e| Error::io(file, e))
}

/// The projects, the most recently opened first.
pub fn list(file: &Path) -> Vec<RecentView> {
    let mut list = load(file);

    list.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    list.into_iter().map(|recent| RecentView { exists: Path::new(&recent.path).is_dir(), recent }).collect()
}

/// Notes that a project was opened just now.
pub fn touch(file: &Path, path: &Path, name: &str) -> Result<()> {
    let key = path.to_string_lossy().to_string();
    let mut list = load(file);

    list.retain(|r| !same(&r.path, &key));
    list.push(Recent { path: key, name: name.to_string(), last_opened: now() });
    list.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    list.truncate(MAX_RECENT);
    store(file, &list)
}

/// Takes a project off the list (the folder is not touched).
pub fn forget(file: &Path, path: &str) -> Result<()> {
    let mut list = load(file);

    list.retain(|r| !same(&r.path, path));
    store(file, &list)
}

fn same(a: &str, b: &str) -> bool {
    PathBuf::from(a) == PathBuf::from(b) || a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_latest_first_and_forgets() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("recent.json");

        touch(&file, &dir.path().join("a"), "A").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        touch(&file, &dir.path().join("b"), "B").unwrap();

        assert_eq!(list(&file).iter().map(|r| r.recent.name.as_str()).collect::<Vec<_>>(), vec!["B", "A"]);

        std::thread::sleep(std::time::Duration::from_millis(1100));
        touch(&file, &dir.path().join("a"), "A").unwrap();

        assert_eq!(list(&file).len(), 2);
        assert_eq!(list(&file)[0].recent.name, "A");
        assert!(!list(&file)[0].exists);

        forget(&file, &dir.path().join("a").to_string_lossy()).unwrap();
        assert_eq!(list(&file).len(), 1);
    }
}
