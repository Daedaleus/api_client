//! Test helpers: a fresh copy of the example collection per test.

use std::path::{Path, PathBuf};

use ac_domain::collection::{Collection, Item, Request};
use tempfile::TempDir;

/// A copy of `fixtures/shop-api` in a temporary folder (deleted with the
/// returned guard) and the path of the copy.
pub fn shop_api() -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("temporary folder");
    let root = temp.path().join("shop-api");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/fixtures/shop-api"),
        &root,
    );
    (temp, root)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// File contents as text, line endings normalised.
pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

/// The request named `name` anywhere in the tree.
pub fn request<'a>(collection: &'a Collection, name: &str) -> &'a Request {
    fn find<'a>(items: &'a [Item], name: &str) -> Option<&'a Request> {
        items.iter().find_map(|item| match item {
            Item::Request(request) if request.name == name => Some(request),
            Item::Folder(folder) => find(&folder.items, name),
            Item::Request(_) => None,
        })
    }
    find(&collection.items, name).unwrap_or_else(|| panic!("no request {name:?}"))
}

/// Names of `items`, folders marked with a trailing `/`.
pub fn tree_names(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            Item::Folder(folder) => format!("{}/", folder.name),
            Item::Request(request) => request.name.clone(),
        })
        .collect()
}
