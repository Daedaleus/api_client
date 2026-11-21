//! Reading the text files of a folder tree, for importing collections that
//! are folders (Bruno). Hidden folders (`.git`) and `node_modules` are left
//! out; nothing is written.

use std::path::Path;

use ac_app::ports::TextFile;

use crate::error::StorageError;

/// Folders never read.
const SKIPPED_DIRS: [&str; 1] = ["node_modules"];
/// More files than this is no collection to import (a wrong folder).
const MAX_FILES: usize = 10_000;

/// Adds the files below `dir` with one of `extensions` to `files`, paths
/// relative to `root` with `/`.
pub(crate) fn read_tree(
    root: &Path,
    dir: &Path,
    extensions: &[&str],
    files: &mut Vec<TextFile>,
) -> Result<(), StorageError> {
    let entries = std::fs::read_dir(dir).map_err(|error| StorageError::io(dir, error))?;
    let mut paths: Vec<_> = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|error| StorageError::io(dir, error))?;
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned());
        if path.is_dir() {
            if !name.starts_with('.') && !SKIPPED_DIRS.contains(&name.as_str()) {
                read_tree(root, &path, extensions, files)?;
            }
            continue;
        }
        let wanted = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension));
        if !wanted {
            continue;
        }
        if files.len() >= MAX_FILES {
            return Err(StorageError::invalid(
                root,
                "folder",
                "too many files to import",
            ));
        }
        let text =
            std::fs::read_to_string(&path).map_err(|error| StorageError::io(&path, error))?;
        let relative = path.strip_prefix(root).unwrap_or(&path);
        files.push(TextFile {
            path: relative.to_string_lossy().replace('\\', "/"),
            text,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/tree.rs"]
mod tests;
