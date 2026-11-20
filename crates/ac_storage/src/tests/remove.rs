use std::path::PathBuf;

use crate::load_collection;
use crate::test_support::{read, shop_api, tree_names};

use super::*;

/// Moves into a test folder instead of the real system trash.
fn into(trash: &Path) -> impl FnOnce(&Path) -> std::io::Result<()> + '_ {
    move |path| std::fs::rename(path, trash.join(path.file_name().unwrap()))
}

#[test]
fn removed_items_leave_the_tree_and_the_order() {
    let (temp, root) = shop_api();
    let trash: PathBuf = temp.path().join("trash");
    std::fs::create_dir(&trash).unwrap();

    remove_item(&root.join("health.toml"), into(&trash)).unwrap();
    remove_item(&root.join("auth"), into(&trash)).unwrap();

    assert!(trash.join("health.toml").is_file());
    assert!(trash.join("auth").join("login.toml").is_file());
    assert!(!read(&root.join("collection.toml")).contains("health.toml"));
    let loaded = load_collection(&root).unwrap();
    assert_eq!(tree_names(&loaded.collection.items), ["Products/"]);
}

#[test]
fn failed_discards_change_nothing() {
    let (_temp, root) = shop_api();
    let before = read(&root.join("collection.toml"));
    let error = remove_item(&root.join("health.toml"), |_| {
        Err(std::io::Error::other("trash unavailable"))
    })
    .unwrap_err();
    assert!(error.to_string().contains("trash unavailable"), "{error}");
    assert!(root.join("health.toml").is_file());
    assert_eq!(read(&root.join("collection.toml")), before);
}
