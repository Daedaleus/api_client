//! Bruno collections (a folder with `bruno.json` and `.bru` files) as
//! collections.
//!
//! Taken over: folders and requests in Bruno's order (`seq`), methods, URLs
//! (`:param` path parameters become `{{param}}` request variables), headers,
//! bodies (JSON, text, XML, URL-encoded, multipart with files, file), auth
//! (Basic, Bearer, API key, none, inherit), request, folder and collection
//! variables, and the environments – secret variables without their value,
//! which Bruno keeps outside the files. Relative file paths are made
//! absolute (they were relative to the Bruno collection). What has no
//! counterpart is reported.
//!
//! - this file: the tree, collection and environments.
//! - `bru`: the `.bru` text format.
//! - `request`: one `.bru` request file.

mod bru;
mod request;

use std::collections::BTreeMap;
use std::path::Path;

use ac_domain::collection::{Collection, Folder, Item, ItemId};
use ac_domain::environment::{Environment, Variable};
use ac_domain::tls::CertificateCheck;

use crate::import::{Imported, Skipped, SkippedKind};
use bru::{Block, blocks, find};

/// The file that marks a Bruno collection.
pub const MARKER: &str = "bruno.json";
/// Extension of Bruno's files.
pub const EXTENSION: &str = "bru";

const COLLECTION_FILE: &str = "collection.bru";
const FOLDER_FILE: &str = "folder.bru";
const ENVIRONMENTS: &str = "environments";

/// A text file of the Bruno collection, by its path relative to the
/// collection folder (with `/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrunoFile {
    /// Path relative to the collection folder, with `/`.
    pub path: String,
    /// Its text.
    pub text: String,
}

/// Converts the files of a Bruno collection in `root`.
#[must_use]
pub fn from_files(root: &Path, files: &[BrunoFile]) -> Imported {
    let by_path: BTreeMap<&str, &str> = files
        .iter()
        .map(|file| (file.path.as_str(), file.text.as_str()))
        .collect();
    let name = by_path
        .get(MARKER)
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|json| json.get("name")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "Bruno".to_owned());
    let mut skipped = Vec::new();
    let settings = by_path
        .get(COLLECTION_FILE)
        .map_or_default(|text| blocks(text));
    let defaults = request::defaults(&settings, &name, &mut skipped);
    let mut tree = Tree::default();
    for file in files {
        tree.add(&file.path);
    }
    let items = tree.items("", &by_path, root, &mut skipped);
    let environments = files
        .iter()
        .filter_map(|file| {
            let stem = file
                .path
                .strip_prefix(ENVIRONMENTS)?
                .strip_prefix('/')?
                .strip_suffix(".bru")?;
            (!stem.contains('/')).then(|| environment(stem, &file.text))
        })
        .collect();
    Imported {
        collection: Collection {
            scripts: ac_domain::script::Scripts::default(),
            id: ItemId::new(),
            name,
            items,
            variables: defaults.variables,
            auth: defaults.auth,
            certificate_check: CertificateCheck::Inherit,
        },
        environments,
        skipped,
    }
}

/// Folders and request files by directory.
#[derive(Default)]
struct Tree {
    /// Directory (relative, `/`, root = "") → request files and subfolders.
    dirs: BTreeMap<String, (Vec<String>, Vec<String>)>,
}

impl Tree {
    fn add(&mut self, path: &str) {
        let Some(stem) = path.strip_suffix(".bru") else {
            return;
        };
        let (dir, file) = stem.rsplit_once('/').unwrap_or(("", stem));
        let top = dir.split('/').next().unwrap_or_default();
        if file == "folder" || file == "collection" || top == ENVIRONMENTS {
            return;
        }
        self.dirs
            .entry(dir.to_owned())
            .or_default()
            .0
            .push(path.to_owned());
        // Register every directory on the way with its parent.
        let mut child = dir.to_owned();
        while !child.is_empty() {
            let parent = child
                .rsplit_once('/')
                .map_or("", |(parent, _)| parent)
                .to_owned();
            let subfolders = &mut self.dirs.entry(parent.clone()).or_default().1;
            if !subfolders.contains(&child) {
                subfolders.push(child.clone());
            }
            child = parent;
        }
    }

    /// The items of `dir`, sorted by Bruno's `seq` (then name).
    fn items(
        &self,
        dir: &str,
        files: &BTreeMap<&str, &str>,
        root: &Path,
        skipped: &mut Vec<Skipped>,
    ) -> Vec<Item> {
        let Some((requests, folders)) = self.dirs.get(dir) else {
            return Vec::new();
        };
        let mut items: Vec<(u32, Item)> = Vec::new();
        for path in requests {
            let text = files.get(path.as_str()).copied().unwrap_or_default();
            let (seq, request) = request::request(path, &blocks(text), root, skipped);
            items.push((seq, Item::Request(request)));
        }
        for folder_dir in folders {
            let settings_path = format!("{folder_dir}/{FOLDER_FILE}");
            let settings: Vec<Block> = files
                .get(settings_path.as_str())
                .map_or_default(|text| blocks(text));
            let meta = find(&settings, "meta");
            let name = meta.and_then(|meta| meta.get("name")).unwrap_or_else(|| {
                folder_dir
                    .rsplit('/')
                    .next()
                    .unwrap_or(folder_dir)
                    .to_owned()
            });
            let seq = seq_of(meta);
            let defaults = request::defaults(&settings, &name, skipped);
            items.push((
                seq,
                Item::Folder(Folder {
                    scripts: ac_domain::script::Scripts::default(),
                    id: ItemId::new(),
                    items: self.items(folder_dir, files, root, skipped),
                    variables: defaults.variables,
                    auth: defaults.auth,
                    certificate_check: CertificateCheck::Inherit,
                    name,
                }),
            ));
        }
        items.sort_by(|(left_seq, left), (right_seq, right)| {
            left_seq
                .cmp(right_seq)
                .then_with(|| left.name().to_lowercase().cmp(&right.name().to_lowercase()))
        });
        items.into_iter().map(|(_, item)| item).collect()
    }
}

/// `seq` of a `meta` block; items without one go last.
fn seq_of(meta: Option<&Block>) -> u32 {
    meta.and_then(|meta| meta.get("seq"))
        .and_then(|seq| seq.parse().ok())
        .unwrap_or(u32::MAX)
}

/// An environment file: `vars { … }` and the names in `vars:secret [ … ]`.
fn environment(name: &str, text: &str) -> Environment {
    let blocks = blocks(text);
    let mut environment = Environment::new(name);
    if let Some(vars) = find(&blocks, "vars") {
        environment.variables = vars
            .entries()
            .into_iter()
            .map(|entry| Variable {
                enabled: entry.enabled,
                ..Variable::plain(entry.key, entry.value)
            })
            .collect();
    }
    if let Some(secrets) = find(&blocks, "vars:secret") {
        for item in secrets.items() {
            let (enabled, name) = match item.strip_prefix('~') {
                Some(name) => (false, name),
                None => (true, item.as_str()),
            };
            environment.variables.push(Variable {
                name: name.to_owned(),
                value: String::new(),
                enabled,
                secret: true,
            });
        }
    }
    environment
}

/// Shortcut for a report.
fn skipped(item: &str, what: SkippedKind) -> Skipped {
    Skipped {
        item: item.to_owned(),
        what,
    }
}

#[cfg(test)]
#[path = "../tests/bruno.rs"]
mod tests;
