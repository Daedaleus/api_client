use crate::test_support::{read, shop_api};

use super::*;

#[test]
fn secrets_files_sit_next_to_their_environment_by_name() {
    let path = Path::new("api").join("environments").join("dev.toml");
    assert_eq!(
        secrets_path(&path),
        Path::new("api").join("secrets").join("dev.age")
    );
}

#[test]
fn reads_and_writes_encrypted_secrets() {
    let (_temp, root) = shop_api();
    let dev = root.join("environments").join("dev.toml");
    assert_eq!(read_secrets(&dev).unwrap(), None);

    let armored = b"-----BEGIN AGE ENCRYPTED FILE-----\nabc\n-----END AGE ENCRYPTED FILE-----\n";
    write_secrets(&dev, armored).unwrap();
    assert_eq!(read_secrets(&dev).unwrap().as_deref(), Some(&armored[..]));
    assert!(root.join("secrets").join("dev.age").is_file());

    let error = write_secrets(&dev, &[0xff, 0xfe]).unwrap_err();
    assert!(matches!(error, StorageError::Invalid { .. }), "{error}");

    std::fs::create_dir_all(root.join("secrets").join("dir.age")).unwrap();
    let dir = root.join("environments").join("dir.toml");
    assert!(read_secrets(&dir).is_err());
}

#[test]
fn recipients_are_listed_without_comments_and_updated_keeping_them() {
    let (_temp, root) = shop_api();
    assert_eq!(read_recipients(&root).unwrap(), Vec::<String>::new());

    write_recipients(&root, &["age1alice".to_owned()]).unwrap();
    let text = read(&root.join(".age-recipients"));
    assert!(text.starts_with("# Public keys"), "{text}");
    assert_eq!(read_recipients(&root).unwrap(), ["age1alice"]);

    std::fs::write(
        root.join(".age-recipients"),
        "# team\nage1alice\n\n  ssh-ed25519 AAAA bob  \n",
    )
    .unwrap();
    assert_eq!(
        read_recipients(&root).unwrap(),
        ["age1alice", "ssh-ed25519 AAAA bob"]
    );
    write_recipients(
        &root,
        &[
            "ssh-ed25519 AAAA bob".to_owned(),
            " age1eve ".to_owned(),
            "age1eve".to_owned(),
        ],
    )
    .unwrap();
    assert_eq!(
        read(&root.join(".age-recipients")),
        "# team\n\n  ssh-ed25519 AAAA bob  \nage1eve\n"
    );

    std::fs::write(root.join(".age-recipients"), "age1eve\nage1eve\n").unwrap();
    write_recipients(&root, &["age1eve".to_owned()]).unwrap();
    assert_eq!(read(&root.join(".age-recipients")), "age1eve\n");

    std::fs::create_dir_all(root.join("dir").join(".age-recipients")).unwrap();
    assert!(read_recipients(&root.join("dir")).is_err());
}
