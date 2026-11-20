use crate::load_collection;
use crate::test_support::{read, shop_api};
use ac_domain::environment::Variable;

use super::*;

#[test]
fn environments_are_loaded_with_the_collection() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();
    assert_eq!(loaded.environments.len(), 1);
    let dev = &loaded.environments[0];
    assert_eq!(dev.name, "Dev");
    assert_eq!(
        dev.variables,
        [
            Variable {
                name: "baseUrl".into(),
                value: "http://localhost:8080".into(),
                enabled: true,
                secret: false,
            },
            Variable {
                name: "debug".into(),
                value: "1".into(),
                enabled: false,
                secret: false,
            },
        ]
    );
    let entry = &loaded.entries[&dev.id];
    assert_eq!(entry.path, root.join("environments").join("dev.toml"));
    assert_eq!(entry.kind, crate::EntryKind::Environment);
}

#[test]
fn saving_unchanged_keeps_the_file_and_changes_touch_only_what_differs() {
    let (_temp, root) = shop_api();
    let path = root.join("environments").join("dev.toml");
    let before = read(&path);
    let (mut dev, read_only) = read_environment(&path).unwrap();
    assert!(!read_only);
    save_environment(&path, &dev).unwrap();
    assert_eq!(read(&path), before);

    dev.name = "Development".into();
    save_environment(&path, &dev).unwrap();
    assert_eq!(read(&path), before.replace("\"Dev\"", "\"Development\""));

    dev.variables[1].enabled = true;
    save_environment(&path, &dev).unwrap();
    let text = read(&path);
    assert!(text.starts_with("# Local development"), "{text}");
    assert_eq!(read_environment(&path).unwrap().0, dev);
}

#[test]
fn new_environments_get_a_file_in_the_environments_folder() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("api");
    let mut staging = Environment::new("Staging EU");
    staging.variables.push(Variable {
        name: "baseUrl".into(),
        value: "https://staging.example.com".into(),
        enabled: true,
        secret: false,
    });
    let path = create_environment(&root, &staging).unwrap();
    assert_eq!(path, root.join("environments").join("staging-eu.toml"));
    assert_eq!(read_environment(&path).unwrap().0, staging);

    let second = create_environment(&root, &Environment::new("Staging EU")).unwrap();
    assert_eq!(second, root.join("environments").join("staging-eu-2.toml"));
}

#[test]
fn damaged_newer_and_missing_files() {
    let (_temp, root) = shop_api();
    let dir = root.join("environments");
    std::fs::write(dir.join("broken.toml"), "nope [").unwrap();
    std::fs::write(
        dir.join("future.toml"),
        "format = \"api-client/environment\"\nversion = 3\nname = \"Future\"\n",
    )
    .unwrap();
    let loaded = load_collection(&root).unwrap();
    assert_eq!(loaded.environments.len(), 2);
    assert!(
        loaded
            .problems
            .iter()
            .any(|problem| problem.error.to_string().contains("broken.toml"))
    );
    let future = loaded
        .environments
        .iter()
        .find(|e| e.name == "Future")
        .unwrap();
    assert!(loaded.entries[&future.id].read_only);

    let error = save_environment(&dir.join("future.toml"), future).unwrap_err();
    assert!(matches!(error, StorageError::ReadOnly { .. }), "{error}");
    let error = save_environment(&dir.join("broken.toml"), future).unwrap_err();
    assert!(matches!(error, StorageError::Toml { .. }), "{error}");
    let error = save_environment(&root.join("nope").join("x.toml"), future).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
    let error = create_environment(&root.join("health.toml"), future).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
}

#[test]
fn files_without_name_use_the_file_name() {
    let (_temp, root) = shop_api();
    let path = root.join("environments").join("qa.toml");
    std::fs::write(&path, "format = \"api-client/environment\"\nversion = 1\n").unwrap();
    let (qa, _) = read_environment(&path).unwrap();
    assert_eq!(qa.name, "qa");
    assert_eq!(qa.variables, vec![]);
}

#[test]
fn secret_variables_are_marked_and_never_get_a_value_in_the_file() {
    let (_temp, root) = shop_api();
    let path = root.join("environments").join("dev.toml");
    let (mut dev, _) = read_environment(&path).unwrap();
    dev.variables.push(Variable {
        name: "token".into(),
        value: "s3cr3t".into(),
        enabled: true,
        secret: true,
    });
    save_environment(&path, &dev).unwrap();
    let text = read(&path);
    assert!(text.contains("secret = true"), "{text}");
    assert!(!text.contains("s3cr3t"), "{text}");
    let (reread, _) = read_environment(&path).unwrap();
    assert!(reread.variables[2].secret);
    assert_eq!(reread.variables[2].value, "");

    // A plain value next to `secret = true` (hand edit) is ignored.
    std::fs::write(
        &path,
        text.replace("secret = true", "secret = true\nvalue = \"leak\""),
    )
    .unwrap();
    let (edited, _) = read_environment(&path).unwrap();
    assert_eq!(edited.variables[2].value, "");
}
