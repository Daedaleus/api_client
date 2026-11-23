use super::*;

fn example(name: &str, status: u16, body: &str) -> ResponseExample {
    ResponseExample {
        name: name.into(),
        status: StatusCode::new(status).unwrap(),
        headers: vec![Header::new("Content-Type", "application/json")],
        body: body.into(),
    }
}

#[test]
fn examples_live_next_to_the_request() {
    assert_eq!(
        path_for(Path::new("api/login.toml")),
        Path::new("api/login.examples.toml")
    );
    assert!(is_examples_file("login.examples.toml"));
    assert!(!is_examples_file("login.toml"));
    assert!(!is_examples_file("examples.txt"));
}

#[test]
fn examples_round_trip_and_none_remove_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let request = temp.path().join("login.toml");
    assert_eq!(read(&request).unwrap(), []);
    let examples = vec![
        example("OK", 200, "{\n  \"token\": \"abc\"\n}\n"),
        ResponseExample {
            headers: Vec::new(),
            ..example("Not found", 404, "")
        },
    ];
    write(&request, &examples).unwrap();
    let text = std::fs::read_to_string(path_for(&request)).unwrap();
    assert!(
        text.starts_with("format = \"api-client/examples\"\nversion = 1\n"),
        "{text}"
    );
    assert!(text.contains("[[example.headers]]"), "{text}");
    assert_eq!(read(&request).unwrap(), examples);

    write(&request, &[]).unwrap();
    assert!(!path_for(&request).exists());
    write(&request, &[]).unwrap();
}

#[test]
fn damaged_files_are_reported_and_kept_aside() {
    let temp = tempfile::tempdir().unwrap();
    let request = temp.path().join("login.toml");
    for damaged in [
        "not toml [",
        "format = \"api-client/examples\"\nversion = 1\nexample = 1\n",
        "format = \"api-client/examples\"\nversion = 1\n[[example]]\nstatus = 42\n",
        "format = \"api-client/examples\"\nversion = 1\n[[example]]\nstatus = 200\nheaders = 1\n",
    ] {
        std::fs::write(path_for(&request), damaged).unwrap();
        assert!(read(&request).is_err(), "{damaged}");
    }
    write(&request, &[example("OK", 200, "{}")]).unwrap();
    assert_eq!(read(&request).unwrap().len(), 1);
    let backups = std::fs::read_dir(temp.path())
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".bak")
        })
        .count();
    assert_eq!(backups, 1);
}

#[test]
fn examples_are_loaded_moved_and_never_taken_for_requests() {
    let (_temp, root) = crate::test_support::shop_api();
    let login = root.join("auth").join("login.toml");
    // The test collection has damaged files of its own.
    let known_problems = crate::load_collection(&root).unwrap().problems.len();
    write(&login, &[example("OK", 200, "{}")]).unwrap();
    let loaded = crate::load_collection(&root).unwrap();
    assert_eq!(
        crate::test_support::tree_names(&loaded.collection.items),
        crate::test_support::tree_names(&crate::load_collection(&root).unwrap().collection.items)
    );
    let request = loaded
        .collection
        .items
        .iter()
        .find_map(|item| match item {
            ac_domain::collection::Item::Folder(folder) => {
                folder.items.iter().find_map(|item| match item {
                    ac_domain::collection::Item::Request(request) if request.name == "Login" => {
                        Some(request.clone())
                    }
                    _ => None,
                })
            }
            ac_domain::collection::Item::Request(_) => None,
        })
        .unwrap();
    assert_eq!(request.examples.len(), 1);
    assert_eq!(
        loaded.problems.len(),
        known_problems,
        "{:?}",
        loaded.problems
    );
    let names = crate::load::entry_names(&root.join("auth"), false).unwrap();
    assert!(
        !names.iter().any(|name| is_examples_file(name)),
        "{names:?}"
    );

    // Moved: the examples follow.
    crate::arrange::move_item(&login, &root, 0).unwrap();
    assert!(root.join("login.examples.toml").is_file());
    assert!(!path_for(&login).exists());

    // Damaged examples are a problem of the collection; the request opens.
    std::fs::write(root.join("login.examples.toml"), "not toml [").unwrap();
    let loaded = crate::load_collection(&root).unwrap();
    assert_eq!(loaded.problems.len(), known_problems + 1);

    // A new request (e.g. a duplicate) gets its examples written.
    let mut copy = request.clone();
    copy.name = "Login copy".into();
    let created = crate::create::create_request(&root, &copy).unwrap();
    assert_eq!(read(&created).unwrap().len(), 1);
}
