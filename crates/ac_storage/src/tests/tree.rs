use super::*;

#[test]
fn reads_wanted_files_below_a_folder_but_not_hidden_ones() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for (path, text) in [
        ("bruno.json", "{}"),
        ("a.bru", "a"),
        ("folder/b.bru", "b"),
        ("folder/notes.txt", "skip"),
        (".git/c.bru", "skip"),
        ("node_modules/d.bru", "skip"),
    ] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    let mut files = Vec::new();
    read_tree(root, root, &["bru", "json"], &mut files).unwrap();
    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["a.bru", "bruno.json", "folder/b.bru"]);
    assert_eq!(files[2].text, "b");

    let error = read_tree(root, &root.join("missing"), &["bru"], &mut Vec::new()).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
}
