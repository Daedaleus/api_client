use crate::load_collection;
use crate::test_support::{read, shop_api};

use super::*;

#[test]
fn renaming_a_folder_keeps_its_order_and_comments() {
    let (_temp, root) = shop_api();
    let dir = root.join("products");
    let id: ItemId = "0193a8f2-7c1e-7b40-9d3a-5f2c8e1b4a71".parse().unwrap();
    let before = read(&dir.join("folder.toml"));
    save_folder_settings(&dir, id, "Catalogue").unwrap();
    assert_eq!(
        read(&dir.join("folder.toml")),
        before.replace("\"Products\"", "\"Catalogue\"")
    );
}

#[test]
fn folders_without_settings_get_a_file_with_their_current_id() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();
    let auth = loaded
        .entries
        .iter()
        .find(|(_, entry)| entry.path == root.join("auth"))
        .map(|(id, _)| *id)
        .unwrap();

    save_folder_settings(&root.join("auth"), auth, "Authentication").unwrap();
    let reloaded = load_collection(&root).unwrap();
    assert_eq!(reloaded.entries[&auth].path, root.join("auth"));
    assert!(read(&root.join("auth").join("folder.toml")).contains("name = \"Authentication\""));
}

#[test]
fn damaged_and_newer_settings_are_not_written() {
    let (_temp, root) = shop_api();
    let settings = root.join("products").join("folder.toml");
    for text in ["broken [", "format = \"api-client/folder\"\nversion = 5\n"] {
        std::fs::write(&settings, text).unwrap();
        assert!(save_folder_settings(&root.join("products"), ItemId::new(), "x").is_err());
        assert_eq!(read(&settings), text);
    }
}

#[test]
fn missing_folders_are_io_errors() {
    let (_temp, root) = shop_api();
    let error = save_folder_settings(&root.join("nope"), ItemId::new(), "x").unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
}
