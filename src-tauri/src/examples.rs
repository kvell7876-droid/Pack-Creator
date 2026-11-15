//! The game's own base girls, shipped inside the Pack Creator (`resources/examples.zip`, made by `scripts/make_examples.py`): their models, textures, animations, event
//! bindings and profiles, and the scenes. The scene simulator offers them next to the girls of the pack, and they are models to learn from. Their files are named for the page
//! as `example:<path in the zip>`, and the commands that read a file of the pack read these too.

use std::io::{Cursor, Read};

const ZIP: &[u8] = include_bytes!("../resources/examples.zip");

pub const PREFIX: &str = "example:";

/// The file at a path in the zip.
pub fn bytes(path: &str) -> Option<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(ZIP)).ok()?;
    let mut file = archive.by_name(path).ok()?;
    let mut out = Vec::with_capacity(file.size() as usize);

    file.read_to_end(&mut out).ok()?;
    Some(out)
}

pub fn text(path: &str) -> Option<String> {
    bytes(path).and_then(|b| String::from_utf8(b).ok())
}

/// Every path in the zip.
pub fn names() -> Vec<String> {
    zip::ZipArchive::new(Cursor::new(ZIP)).map(|a| a.file_names().map(String::from).collect()).unwrap_or_default()
}

/// The path in the zip a page's `example:` name stands for.
pub fn strip(rel: &str) -> Option<&str> {
    rel.strip_prefix(PREFIX)
}
