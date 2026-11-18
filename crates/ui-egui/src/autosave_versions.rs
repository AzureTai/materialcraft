//! Write numbered recovery copies and retain the newest copies for each document

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use craft_atomic::{NewFilePermissions, WriteOptions};
use materialcraft_engine::core::Package;
use materialcraft_engine::formats::project::ProjectFiles;
use sha2::{Digest, Sha256};

const EXTENSION: &str = ".mcraft";
const MAX_WRITE_ATTEMPTS: usize = 8;

/// Saved documents share history by normalized path; untitled documents use workspace-local IDs
pub(crate) fn folder(root: &Path, path: Option<&str>, workspace: u64, document: u64) -> PathBuf {
    let key = match path.filter(|path| !path.contains("://")) {
        Some(path) => format!("file-{:x}", Sha256::digest(path.as_bytes())),
        None => format!("untitled-{workspace:016x}-{document}"),
    };
    root.join(key)
}

fn copies(folder: &Path) -> io::Result<impl Iterator<Item = (u64, std::fs::DirEntry)>> {
    Ok(std::fs::read_dir(folder)?.filter_map(|entry| {
        let entry = entry.ok()?;
        let filename = entry.file_name();
        let number = filename.to_str()?.strip_suffix(EXTENSION)?.parse().ok()?;
        entry.file_type().ok()?.is_file().then_some((number, entry))
    }))
}

fn next_path(folder: &Path) -> io::Result<PathBuf> {
    let mut version = copies(folder)?.map(|(number, _)| number).max().unwrap_or(0);
    loop {
        version = version.checked_add(1).ok_or_else(|| io::Error::other("autosave versions exhausted"))?;
        let path = folder.join(format!("{version}{EXTENSION}"));

        // A directory or link can occupy a name without counting toward retention
        match path.symlink_metadata() {
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(path),
            Err(error) => return Err(error),
            Ok(_) => continue,
        }
    }
}

fn prune(folder: &Path, keep: u32) -> io::Result<()> {
    let mut copies: Vec<_> = copies(folder)?.collect();
    copies.sort_unstable_by_key(|(number, _)| *number);
    let surplus = copies.len().saturating_sub(keep.max(1) as usize);

    for (_, entry) in copies.into_iter().take(surplus) {
        match std::fs::remove_file(entry.path()) {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            result => result?,
        }
    }
    Ok(())
}

pub(crate) fn save(folder: &Path, package: &Package, keep: u32) -> io::Result<()> {
    std::fs::create_dir_all(folder)?;
    let files = ProjectFiles::build(package, "autosave.mcraft", true).map_err(io::Error::other)?;
    let options = WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private);

    for _ in 0..MAX_WRITE_ATTEMPTS {
        let path = next_path(folder)?;
        match options.write_new_with(&path, |file| files.write_zip(file).map(|_| ()).map_err(io::Error::other)) {
            Ok(()) => return prune(folder, keep),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("autosave version could not be reserved"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_keeps_distinct_packages_and_ignores_directories() {
        let root = tempfile::tempdir().unwrap();
        let wood = Package::new("Wood");
        let first = folder(root.path(), Some("/first/wood.sbs"), 1, 0);
        let second = folder(root.path(), Some("/second/wood.sbs"), 1, 1);
        let untitled = folder(root.path(), None, 1, 0);
        assert_ne!(untitled, folder(root.path(), None, 2, 0));
        assert_ne!(untitled, folder(root.path(), None, 1, 1));
        assert_eq!(first, folder(root.path(), Some("/first/wood.sbs"), 2, 1));

        for _ in 0..5 {
            save(&first, &wood, 2).unwrap();
        }
        save(&second, &wood, 1).unwrap();
        std::fs::create_dir(first.join("99.mcraft")).unwrap();
        std::fs::rename(first.join("4.mcraft"), first.join("04.mcraft")).unwrap();
        save(&first, &wood, 2).unwrap();

        assert!(!first.join("04.mcraft").exists());
        assert!(first.join("5.mcraft").is_file());
        assert!(first.join("6.mcraft").is_file());
        assert!(first.join("99.mcraft").is_dir());
        assert!(second.join("1.mcraft").is_file());
        let archive = std::fs::read(first.join("6.mcraft")).unwrap();
        assert!(materialcraft_engine::formats::project::read(&archive, "autosave.mcraft").is_ok());
    }

    #[test]
    fn an_occupied_version_name_does_not_block_recovery() {
        let root = tempfile::tempdir().unwrap();
        let occupied = root.path().join("1.mcraft");
        std::fs::create_dir(&occupied).unwrap();

        save(root.path(), &Package::new("Wood"), 1).unwrap();

        assert!(occupied.is_dir());
        let bytes = std::fs::read(root.path().join("2.mcraft")).unwrap();
        assert!(materialcraft_engine::formats::project::read(&bytes, "autosave.mcraft").is_ok());
    }

    #[test]
    fn simultaneous_writes_publish_distinct_versions() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path();
        let barrier = std::sync::Barrier::new(4);

        std::thread::scope(|scope| {
            for number in 0..4 {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    let mut package = Package::new("Same filename");
                    package.graphs.push(materialcraft_engine::core::Graph::new(&format!("Graph {number}")));
                    save(folder, &package, 4).unwrap();
                });
            }
        });

        let mut names = Vec::new();
        for (_, entry) in copies(folder).unwrap() {
            let archive = std::fs::read(entry.path()).unwrap();
            names.push(materialcraft_engine::formats::project::read(&archive, "autosave.mcraft").unwrap().pkg.graphs[0].name.clone());
        }

        names.sort_unstable();
        assert_eq!(names, ["Graph 0", "Graph 1", "Graph 2", "Graph 3"]);
    }
}
