use std::path::Path;

use toml_edit::DocumentMut;

use super::*;

fn parsed(text: &str) -> Result<TransportSettings, StorageError> {
    let doc: DocumentMut = text.parse().unwrap();
    read(doc.as_table(), Path::new("x.toml"))
}

#[test]
fn settings_are_read_and_missing_ones_inherit() {
    assert_eq!(parsed("").unwrap(), TransportSettings::default());
    assert_eq!(
        parsed("follow_redirects = false\ntimeout_seconds = 5").unwrap(),
        TransportSettings {
            redirects: Redirects::Stop,
            timeout_seconds: Some(5),
        }
    );
    assert_eq!(
        parsed("follow_redirects = true").unwrap().redirects,
        Redirects::Follow
    );
    for wrong in [
        "timeout_seconds = 0",
        "timeout_seconds = -1",
        "timeout_seconds = \"5\"",
    ] {
        assert!(parsed(wrong).is_err(), "{wrong}");
    }
}

#[test]
fn only_changes_are_written() {
    let mut doc: DocumentMut = "# keep\nfollow_redirects = true\n".parse().unwrap();
    let before = read(doc.as_table(), Path::new("x.toml")).unwrap();
    write(doc.as_table_mut(), &before, &before);
    assert_eq!(doc.to_string(), "# keep\nfollow_redirects = true\n");

    let after = TransportSettings {
        redirects: Redirects::Stop,
        timeout_seconds: Some(12),
    };
    write(doc.as_table_mut(), &before, &after);
    assert_eq!(
        doc.to_string(),
        "# keep\nfollow_redirects = false\ntimeout_seconds = 12\n"
    );
    write(doc.as_table_mut(), &after, &TransportSettings::default());
    assert_eq!(doc.to_string(), "");
}
