use std::fs;

use super::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn temporary_roots_are_new_folders_below_the_base() {
    let base = tempfile::tempdir().unwrap();
    let first = temporary_root(base.path());
    let second = temporary_root(base.path());
    assert!(first.starts_with(base.path().join(TEMPORARY_DIR)));
    assert_ne!(first, second);
    assert!(!first.exists());
}

#[test]
fn moving_takes_the_whole_folder_and_refuses_occupied_targets() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    write(&from.join("collection.toml"), "name = 'x'");
    write(&from.join("auth/login.toml"), "[request]");

    // An empty target folder is fine, a missing parent is created.
    let empty = dir.path().join("empty");
    fs::create_dir(&empty).unwrap();
    move_collection(&from, &empty).unwrap();
    assert!(!from.exists());
    assert!(empty.join("auth/login.toml").exists());

    let nested = dir.path().join("a/b/c");
    move_collection(&empty, &nested).unwrap();
    assert_eq!(
        fs::read_to_string(nested.join("collection.toml")).unwrap(),
        "name = 'x'"
    );

    let occupied = dir.path().join("occupied");
    write(&occupied.join("keep.txt"), "mine");
    assert!(matches!(
        move_collection(&nested, &occupied),
        Err(StorageError::AlreadyExists { .. })
    ));
    assert_eq!(
        fs::read_to_string(occupied.join("keep.txt")).unwrap(),
        "mine"
    );
    assert!(nested.join("collection.toml").exists());
}

#[test]
fn copying_keeps_every_file() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    write(&from.join("x/y/z.toml"), "z");
    let to = dir.path().join("to");
    copy_dir(&from, &to).unwrap();
    assert_eq!(fs::read_to_string(to.join("x/y/z.toml")).unwrap(), "z");
    assert!(copy_dir(&dir.path().join("missing"), &to).is_err());
}

#[test]
fn only_unsaved_collections_are_deleted() {
    let base = tempfile::tempdir().unwrap();
    let root = temporary_root(base.path());
    write(&root.join("collection.toml"), "x");
    discard(base.path(), &root).unwrap();
    assert!(!root.exists());
    // Already gone is fine.
    discard(base.path(), &root).unwrap();

    let saved = base.path().join("my-collection");
    write(&saved.join("collection.toml"), "x");
    for refused in [
        saved.clone(),
        base.path().join(TEMPORARY_DIR),
        base.path()
            .join(TEMPORARY_DIR)
            .join("..")
            .join("my-collection"),
    ] {
        assert!(
            discard(base.path(), &refused).is_err(),
            "{}",
            refused.display()
        );
    }
    assert!(saved.join("collection.toml").exists());
}
