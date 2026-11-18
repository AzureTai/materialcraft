//! The autosaved copies of a package: numbered files in the autosave folder, the newest kept

use std::path::{Path, PathBuf};

const EXTENSION: &str = "mcraft";

/// The file name stem of `package`'s copies, before the copy's number
fn prefix(package: &str) -> String {
    format!("{}-autosave-", materialcraft_engine::formats::safe_file_name(package))
}

/// The numbers of `package`'s copies in `folder`, lowest first
fn numbers(folder: &Path, package: &str) -> Vec<u64> {
    let prefix = prefix(package);
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };

    let mut numbers: Vec<u64> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|name| name.strip_prefix(&prefix)?.strip_suffix(&format!(".{EXTENSION}"))?.parse().ok())
        .collect();
    numbers.sort_unstable();
    numbers
}

/// Where the next copy of `package` goes: one number past the newest
pub(crate) fn next_path(folder: &Path, package: &str) -> PathBuf {
    let next = numbers(folder, package).last().map_or(1, |newest| newest.saturating_add(1));
    folder.join(format!("{}{next}.{EXTENSION}", prefix(package)))
}

/// Delete all but the newest `keep` copies of `package`; returns how many were deleted
pub(crate) fn prune(folder: &Path, package: &str, keep: u32) -> usize {
    let numbers = numbers(folder, package);
    let surplus = numbers.len().saturating_sub(keep.max(1) as usize);

    numbers.iter().take(surplus).filter(|number| std::fs::remove_file(folder.join(format!("{}{number}.{EXTENSION}", prefix(package)))).is_ok()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_are_numbered_and_only_the_newest_are_kept() {
        let folder = std::env::temp_dir().join(format!("materialcraft-autosave-versions-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();

        for _ in 0..5 {
            std::fs::write(next_path(&folder, "Wood"), b"copy").unwrap();
        }
        std::fs::write(next_path(&folder, "Metal"), b"copy").unwrap();
        assert_eq!(next_path(&folder, "Wood"), folder.join("Wood-autosave-6.mcraft"));

        assert_eq!(prune(&folder, "Wood", 2), 3);
        assert_eq!(numbers(&folder, "Wood"), vec![4, 5]);
        assert_eq!(numbers(&folder, "Metal"), vec![1], "another package's copies stay");
        assert_eq!(prune(&folder, "Wood", 0), 1, "at least one copy is always kept");

        std::fs::remove_dir_all(&folder).unwrap();
    }
}
