//! File and folder names derived from display names.

use std::path::{Path, PathBuf};

use crate::format::EXTENSION;

/// Longest name stem created from a display name.
const MAX_SLUG: usize = 60;
/// Names Windows does not allow as file or folder names.
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Name stem for a display name: lower case ASCII letters and digits, `-`
/// between words, German umlauts transliterated. `fallback` is used when no
/// usable character is left (and as prefix for names Windows reserves).
#[must_use]
pub fn slug(name: &str, fallback: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => slug.push_str("ae"),
            'ö' => slug.push_str("oe"),
            'ü' => slug.push_str("ue"),
            'ß' => slug.push_str("ss"),
            c if c.is_ascii_alphanumeric() => slug.push(c),
            _ if !slug.ends_with('-') => slug.push('-'),
            _ => {}
        }
    }
    let mut slug = slug.trim_matches('-').to_owned();
    // Only ASCII is left, so cutting at a byte index is safe.
    slug.truncate(MAX_SLUG);
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        fallback.to_owned()
    } else if WINDOWS_RESERVED.contains(&slug.as_str()) {
        format!("{fallback}-{slug}")
    } else {
        slug
    }
}

/// `dir/stem.toml`, or `stem-2.toml`, `stem-3.toml` … if taken.
pub(crate) fn unique_file(dir: &Path, stem: &str) -> PathBuf {
    unique(dir, stem, &format!(".{EXTENSION}"))
}

/// `dir/stem`, or `stem-2`, `stem-3` … if taken.
pub(crate) fn unique_dir(dir: &Path, stem: &str) -> PathBuf {
    unique(dir, stem, "")
}

fn unique(dir: &Path, stem: &str, suffix: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}{suffix}"));
    let mut number = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{number}{suffix}"));
        number += 1;
    }
    path
}

/// The last component of `path` without extension, as text.
pub(crate) fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// The last component of `path` as text.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "tests/names.rs"]
mod tests;
