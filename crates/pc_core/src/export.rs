//! Getting a pack out of the tool: as one zip to share, or copied into a world's `datapacks` folder to try in the game.

use crate::check::walk;
use crate::error::{Error, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes the pack as a zip with its files at the top, the way Minecraft reads a data pack. The tool's own folder and hidden files are left out. Returns how many files went in.
pub fn zip_pack(root: &Path, destination: &Path) -> Result<usize> {
    if destination.starts_with(root) {
        return Err(Error::invalid("Save the zip outside the pack's folder, or it would be packed into itself."));
    }

    let file = fs::File::create(destination).map_err(|e| Error::io(destination, e))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;

    for rel in walk(root) {
        let bytes = fs::read(root.join(&rel)).map_err(|e| Error::io(root.join(&rel), e))?;

        writer.start_file(rel.as_str(), options).map_err(|e| Error::Other(format!("{rel} could not be added to the zip: {e}")))?;
        writer.write_all(&bytes).map_err(|e| Error::io(destination, e))?;
        count += 1;
    }

    writer.finish().map_err(|e| Error::Other(format!("The zip could not be finished: {e}")))?;
    Ok(count)
}

/// Copies the pack into `<world>/datapacks/<folder name>`, replacing an earlier copy of it, so a world can run it after `/reload`. Returns the folder it was copied to.
pub fn install(root: &Path, world: &Path, folder: &str) -> Result<PathBuf> {
    if !world.join("level.dat").is_file() {
        return Err(Error::invalid(format!("{} does not look like a world: it has no level.dat.", world.display())));
    }

    let target = world.join("datapacks").join(folder);

    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| Error::io(&target, e))?;
    }

    for rel in walk(root) {
        let to = target.join(&rel);

        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }

        fs::copy(root.join(&rel), &to).map_err(|e| Error::io(&to, e))?;
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("pack.mcmeta"), "{}").unwrap();
        fs::create_dir_all(dir.path().join("data/x/girls")).unwrap();
        fs::write(dir.path().join("data/x/girls/a.json"), "{}").unwrap();
        fs::create_dir_all(dir.path().join(".packcreator")).unwrap();
        fs::write(dir.path().join(".packcreator/project.json"), "{}").unwrap();
        dir
    }

    #[test]
    fn zips_the_pack_without_the_tools_files() {
        let dir = pack();
        let out = tempfile::tempdir().unwrap();
        let zip_path = out.path().join("p.zip");

        assert_eq!(zip_pack(dir.path(), &zip_path).unwrap(), 2);

        let mut archive = zip::ZipArchive::new(fs::File::open(&zip_path).unwrap()).unwrap();
        let mut names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().to_string()).collect();

        names.sort();
        assert_eq!(names, vec!["data/x/girls/a.json", "pack.mcmeta"]);
        assert!(zip_pack(dir.path(), &dir.path().join("self.zip")).is_err());
    }

    #[test]
    fn installs_into_a_world_and_replaces_the_old_copy() {
        let dir = pack();
        let world = tempfile::tempdir().unwrap();

        assert!(install(dir.path(), world.path(), "mypack").is_err());

        fs::write(world.path().join("level.dat"), "x").unwrap();
        fs::create_dir_all(world.path().join("datapacks/mypack")).unwrap();
        fs::write(world.path().join("datapacks/mypack/stale.txt"), "x").unwrap();

        let target = install(dir.path(), world.path(), "mypack").unwrap();

        assert!(target.join("data/x/girls/a.json").is_file());
        assert!(!target.join("stale.txt").exists());
        assert!(!target.join(".packcreator").exists());
    }
}
