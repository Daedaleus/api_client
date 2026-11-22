use super::*;
use crate::load_collection;
use crate::test_support::{read, shop_api};

/// Names of the top-level items as shown.
fn top_level(root: &Path) -> Vec<String> {
    load_collection(root)
        .unwrap()
        .collection
        .items
        .iter()
        .map(|item| item.name().to_owned())
        .collect()
}

#[test]
fn reordering_writes_the_order_as_shown() {
    let (_temp, root) = shop_api();
    // Shown: Products (listed), Health (listed), then auth (by name).
    assert_eq!(top_level(&root), ["Products", "Health", "auth"]);
    move_item(&root.join("auth"), &root, 0).unwrap();
    assert_eq!(top_level(&root), ["auth", "Products", "Health"]);
    assert!(
        read(&root.join("collection.toml"))
            .contains("order = [\"auth\", \"products\", \"health.toml\"]")
    );
    // Past the end: last.
    move_item(&root.join("auth"), &root, 99).unwrap();
    assert_eq!(top_level(&root), ["Products", "Health", "auth"]);
}

#[test]
fn moving_into_another_folder_keeps_ids() {
    let (_temp, root) = shop_api();
    let before = load_collection(&root).unwrap();
    let login_id = before
        .entries
        .iter()
        .find(|(_, entry)| entry.path == root.join("auth").join("login.toml"))
        .map(|(id, _)| *id)
        .unwrap();
    let auth_id = before
        .entries
        .iter()
        .find(|(_, entry)| entry.path == root.join("auth"))
        .map(|(id, _)| *id)
        .unwrap();

    // A request into the products folder, at the top.
    let moved = move_item(
        &root.join("auth").join("login.toml"),
        &root.join("products"),
        0,
    )
    .unwrap();
    assert_eq!(moved, root.join("products").join("login.toml"));
    let after = load_collection(&root).unwrap();
    assert_eq!(after.entries[&login_id].path, moved);
    assert!(read(&root.join("products").join("folder.toml")).contains("\"login.toml\""));

    // A folder without settings into another folder: it keeps its id.
    let folder = move_item(&root.join("auth"), &root.join("products"), 1).unwrap();
    assert!(folder.join("folder.toml").is_file());
    let after = load_collection(&root).unwrap();
    assert_eq!(after.entries[&auth_id].path, folder);
    assert!(!read(&root.join("collection.toml")).contains("\"auth\""));
}

#[test]
fn taken_names_get_a_suffix_and_folders_stay_out_of_themselves() {
    let (_temp, root) = shop_api();
    std::fs::write(root.join("auth").join("health.toml"), "").unwrap();
    let moved = move_item(&root.join("health.toml"), &root.join("auth"), 0).unwrap();
    assert_eq!(moved, root.join("auth").join("health-2.toml"));

    let error = move_item(&root.join("products"), &root.join("products"), 0).unwrap_err();
    assert!(error.to_string().contains("into itself"), "{error}");
}

#[test]
fn read_only_settings_are_not_written() {
    let (_temp, root) = shop_api();
    let settings = root.join("collection.toml");
    let text = read(&settings).replace("version = 1", "version = 99");
    std::fs::write(&settings, &text).unwrap();
    assert!(matches!(
        move_item(&root.join("auth"), &root, 0),
        Err(StorageError::ReadOnly { .. })
    ));
    assert_eq!(read(&settings), text);
}
