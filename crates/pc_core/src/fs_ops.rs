//! Everything the workspace does to files. Every path comes in relative to the project and is checked to stay inside it, because the paths come from a web page.

use crate::error::{Error, Result};
use crate::layout::{locate, FileKind};
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The largest text file the editor opens.
pub const MAX_EDIT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub name: String,
    /// Relative to the project, written with `/`.
    pub rel: String,
    pub is_dir: bool,
    pub kind: FileKind,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Binary { size: u64 },
    TooLarge { size: u64 },
}

/// The real path of `rel` inside `root`, or why it is refused. Nothing that leaves the project passes: no `..`, no absolute path, no drive.
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf> {
    let path = Path::new(rel);

    /* Every part of the path must be a plain name: no "..", no ".", no empty part, no drive, no back slash. */
    let plain = rel.is_empty() || rel.split('/').all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':') && !part.contains('\\'));

    if !plain || path.is_absolute() || path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(Error::Outside(rel.to_string()));
    }

    let joined = root.join(path);

    /* A link inside the project that leads out of it is outside too. */
    let anchor = if joined.exists() { joined.clone() } else { joined.parent().map(Path::to_path_buf).unwrap_or_else(|| root.to_path_buf()) };

    if let (Ok(real), Ok(base)) = (anchor.canonicalize(), root.canonicalize()) {
        if !real.starts_with(&base) {
            return Err(Error::Outside(rel.to_string()));
        }
    }

    Ok(joined)
}

/// Why this is not a name for a file or a folder, or None when it is.
pub fn name_problem(name: &str) -> Option<String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Some("The name cannot be empty.".into());
    }

    if trimmed != name || name.ends_with('.') {
        return Some("The name cannot start or end with a space, or end with a dot.".into());
    }

    if name.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control()) {
        return Some("The name cannot have any of < > : \" / \\ | ? *".into());
    }

    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();

    if ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "LPT1", "LPT2", "LPT3"].contains(&stem.as_str()) {
        return Some(format!("{stem} is a name Windows keeps for itself."));
    }

    if name.len() > 100 {
        return Some("The name is too long.".into());
    }

    None
}

/// The entries of a folder: folders first, then files, each in alphabetical order. Hidden files and the tool's folder are left out.
pub fn list_dir(root: &Path, rel: &str) -> Result<Vec<Entry>> {
    let dir = resolve(root, rel)?;
    let mut entries = Vec::new();

    for entry in fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        let meta = entry.metadata().map_err(|e| Error::io(entry.path(), e))?;
        let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };

        entries.push(Entry { kind: if meta.is_dir() { FileKind::Other } else { locate(&child).kind }, name, rel: child, is_dir: meta.is_dir(), size: meta.len() });
    }

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

/// What a file holds, for the editor: its text if it is text, else just its size.
pub fn read(root: &Path, rel: &str) -> Result<Content> {
    let path = resolve(root, rel)?;
    let size = fs::metadata(&path).map_err(|e| Error::io(&path, e))?.len();

    if size > MAX_EDIT_BYTES {
        return Ok(Content::TooLarge { size });
    }

    let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;

    Ok(match String::from_utf8(bytes) {
        Ok(text) => Content::Text { text: text.strip_prefix('\u{feff}').map(String::from).unwrap_or(text) },
        Err(_) => Content::Binary { size },
    })
}

/// The bytes of a file, up to `limit` (a picture to show).
pub fn read_bytes(root: &Path, rel: &str, limit: u64) -> Result<Vec<u8>> {
    let path = resolve(root, rel)?;
    let size = fs::metadata(&path).map_err(|e| Error::io(&path, e))?.len();

    if size > limit {
        return Err(Error::invalid(format!("The file is {} MiB, too large to show.", size / 1024 / 1024)));
    }

    fs::read(&path).map_err(|e| Error::io(&path, e))
}

/// Writes a text file so that a crash in the middle leaves the old file, not half a new one: the text goes to a temporary file beside it, which then replaces it.
pub fn write_text(root: &Path, rel: &str, text: &str) -> Result<()> {
    let path = resolve(root, rel)?;
    let temp = path.with_extension(format!("{}.pctmp", path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()));

    fs::write(&temp, text).map_err(|e| Error::io(&temp, e))?;
    fs::rename(&temp, &path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        Error::io(&path, e)
    })
}

/// Writes a binary file the same careful way as `write_text`: a temporary file beside it, then a rename over it.
pub fn write_bytes(root: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    let path = resolve(root, rel)?;
    let temp = path.with_extension(format!("{}.pctmp", path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()));

    fs::write(&temp, bytes).map_err(|e| Error::io(&temp, e))?;
    fs::rename(&temp, &path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        Error::io(&path, e)
    })
}

/// When a file last changed and how big it is, as one string that differs whenever the file does (nothing when it is not there). The page compares these to notice that a
/// file was changed by another program.
pub fn stamp(root: &Path, rel: &str) -> Option<String> {
    let path = resolve(root, rel).ok()?;
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();

    Some(format!("{modified}:{}", meta.len()))
}

/// Makes a new file with `text`. Refuses one that exists.
pub fn create_file(root: &Path, rel: &str, text: &str) -> Result<()> {
    let path = resolve(root, rel)?;

    check_new(&path, rel)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    fs::write(&path, text).map_err(|e| Error::io(&path, e))
}

pub fn create_dir(root: &Path, rel: &str) -> Result<()> {
    let path = resolve(root, rel)?;

    check_new(&path, rel)?;
    fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))
}

fn check_new(path: &Path, rel: &str) -> Result<()> {
    if let Some(problem) = path.file_name().and_then(|n| name_problem(&n.to_string_lossy())) {
        return Err(Error::invalid(problem));
    }

    if path.exists() {
        return Err(Error::invalid(format!("{rel} already exists.")));
    }

    Ok(())
}

pub fn rename(root: &Path, from: &str, to: &str) -> Result<()> {
    let source = resolve(root, from)?;
    let target = resolve(root, to)?;

    check_new(&target, to)?;

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    fs::rename(&source, &target).map_err(|e| Error::io(&source, e))
}

/// Moves a file or folder to the Recycle Bin, so a mistake can be undone from Windows.
pub fn delete(root: &Path, rel: &str) -> Result<()> {
    if rel.is_empty() {
        return Err(Error::invalid("The pack's own folder cannot be deleted from inside it."));
    }

    let path = resolve(root, rel)?;

    trash::delete(&path).map_err(|e| Error::Other(format!("{rel} could not be moved to the Recycle Bin: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_paths_that_leave_the_project() {
        let dir = tempfile::tempdir().unwrap();

        for bad in ["../x", "a/../../x", "/etc/passwd", "C:/x", "a\\b", "a/./b"] {
            assert!(resolve(dir.path(), bad).is_err(), "{bad}");
        }

        assert!(resolve(dir.path(), "data/x/girls/a.json").is_ok());
        assert!(resolve(dir.path(), "").is_ok());
    }

    #[test]
    fn writes_without_leaving_a_temporary_file() {
        let dir = tempfile::tempdir().unwrap();

        create_file(dir.path(), "data/x/a.json", "{}").unwrap();
        write_text(dir.path(), "data/x/a.json", "{\"a\":1}").unwrap();

        assert!(matches!(read(dir.path(), "data/x/a.json").unwrap(), Content::Text { text } if text == "{\"a\":1}"));
        assert_eq!(fs::read_dir(dir.path().join("data/x")).unwrap().count(), 1);
        assert!(create_file(dir.path(), "data/x/a.json", "").is_err());
    }

    #[test]
    fn lists_folders_first_and_hides_the_tools_own() {
        let dir = tempfile::tempdir().unwrap();

        create_file(dir.path(), "b.txt", "").unwrap();
        create_file(dir.path(), "a/x.json", "").unwrap();
        create_file(dir.path(), ".packcreator/project.json", "").unwrap();

        let names: Vec<_> = list_dir(dir.path(), "").unwrap().into_iter().map(|e| e.name).collect();

        assert_eq!(names, vec!["a", "b.txt"]);
    }

    #[test]
    fn renames_and_checks_names() {
        let dir = tempfile::tempdir().unwrap();

        create_file(dir.path(), "a.json", "x").unwrap();
        rename(dir.path(), "a.json", "sub/b.json").unwrap();

        assert!(dir.path().join("sub/b.json").is_file());
        assert!(name_problem("a:b").is_some());
        assert!(name_problem("con.json").is_some());
        assert!(name_problem("fine_name.json").is_none());
        assert!(create_file(dir.path(), "bad?.json", "").is_err());
    }

    #[test]
    fn a_stamp_changes_when_the_file_does() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(stamp(dir.path(), "a.txt"), None);
        create_file(dir.path(), "a.txt", "one").unwrap();

        let first = stamp(dir.path(), "a.txt").unwrap();

        write_text(dir.path(), "a.txt", "three").unwrap();
        assert_ne!(stamp(dir.path(), "a.txt").unwrap(), first);
    }

    #[test]
    fn tells_text_from_binary() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("a.bin"), [0xff, 0xfe, 0x00]).unwrap();
        assert!(matches!(read(dir.path(), "a.bin").unwrap(), Content::Binary { size: 3 }));
    }
}
