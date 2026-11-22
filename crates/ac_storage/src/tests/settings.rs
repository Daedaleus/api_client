use crate::load_collection;
use crate::test_support::{read, shop_api};

use ac_domain::collection::RequestDefaults;
use ac_domain::environment::Variable;

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

/// Variables only, authentication inherited.
fn with_variables(variables: &[Variable]) -> RequestDefaults {
    RequestDefaults {
        transport: ac_domain::transport::TransportSettings::default(),
        scripts: ac_domain::script::Scripts::default(),
        certificate_check: ac_domain::tls::CertificateCheck::Inherit,
        variables: variables.to_vec(),
        auth: ac_domain::auth::Auth::Inherit,
    }
}

#[test]
fn collection_and_folder_variables_are_written_in_place() {
    let (_temp, root) = shop_api();
    let collection_before = read(&root.join("collection.toml"));
    save_collection_defaults(
        &root,
        &with_variables(&[Variable::plain("host", "api.test")]),
    )
    .unwrap();
    let text = read(&root.join("collection.toml"));
    assert!(text.starts_with(&collection_before), "{text}");
    assert!(text.contains("[[variables]]\nname = \"host\""), "{text}");

    let loaded = load_collection(&root).unwrap();
    let auth = loaded
        .entries
        .iter()
        .find(|(_, entry)| entry.path == root.join("auth"))
        .map(|(id, _)| *id)
        .unwrap();
    // No folder.toml yet: it is created with the folder's id and name.
    save_folder_defaults(
        &root.join("auth"),
        auth,
        "auth",
        &with_variables(&[Variable::plain("v", "2")]),
    )
    .unwrap();
    let products: ItemId = "0193a8f2-7c1e-7b40-9d3a-5f2c8e1b4a71".parse().unwrap();
    save_folder_defaults(
        &root.join("products"),
        products,
        "Products",
        &with_variables(&[Variable::plain("page", "1")]),
    )
    .unwrap();

    let reloaded = load_collection(&root).unwrap();
    assert_eq!(
        reloaded.collection.variables,
        [Variable::plain("host", "api.test")]
    );
    assert_eq!(reloaded.entries[&auth].path, root.join("auth"));
    let folder_variables = |id| match reloaded.collection.find(id) {
        Some(ac_domain::collection::Item::Folder(folder)) => folder.variables.clone(),
        other => panic!("expected a folder, got {other:?}"),
    };
    assert_eq!(folder_variables(auth), [Variable::plain("v", "2")]);
    assert_eq!(folder_variables(products), [Variable::plain("page", "1")]);
    assert!(read(&root.join("products").join("folder.toml")).contains("order"));
}

#[test]
fn collection_variables_need_a_writable_collection_file() {
    let (_temp, root) = shop_api();
    let error = save_collection_defaults(&root.join("nope"), &with_variables(&[])).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
    let settings = root.join("collection.toml");
    std::fs::write(
        &settings,
        "format = \"api-client/collection\"\nversion = 7\n",
    )
    .unwrap();
    let error = save_collection_defaults(&root, &with_variables(&[])).unwrap_err();
    assert!(matches!(error, StorageError::ReadOnly { .. }), "{error}");
}

#[test]
fn folder_and_collection_scripts_are_saved_and_loaded() {
    let (_temp, root) = shop_api();
    let scripts = ac_domain::script::Scripts {
        pre_request: "req.setHeader('X-Team', 'a');".into(),
        post_response: "log(res.status);".into(),
    };
    let defaults = RequestDefaults {
        scripts: scripts.clone(),
        ..RequestDefaults::default()
    };
    let dir = root.join("products");
    let id: ItemId = "0193a8f2-7c1e-7b40-9d3a-5f2c8e1b4a71".parse().unwrap();
    save_folder_defaults(&dir, id, "Products", &defaults).unwrap();
    save_collection_defaults(&root, &defaults).unwrap();
    assert!(read(&dir.join("folder.toml")).contains("[scripts]"));
    assert!(read(&root.join("collection.toml")).contains("[scripts]"));

    let loaded = load_collection(&root).unwrap();
    assert_eq!(loaded.collection.scripts, scripts);
    let Some(ac_domain::collection::Item::Folder(folder)) = loaded.collection.find(id) else {
        panic!("folder missing");
    };
    assert_eq!(folder.scripts, scripts);

    // Removed again: no table left.
    save_collection_defaults(&root, &RequestDefaults::default()).unwrap();
    assert!(!read(&root.join("collection.toml")).contains("[scripts]"));
}
