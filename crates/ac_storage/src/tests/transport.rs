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
        parsed("follow_redirects = false\ntimeout_seconds = 5\ncookies = false").unwrap(),
        TransportSettings {
            redirects: Redirects::Stop,
            timeout_seconds: Some(5),
            cookies: Cookies::Off,
        }
    );
    assert_eq!(parsed("cookies = true").unwrap().cookies, Cookies::On);
    assert_eq!(
        parsed("follow_redirects = true").unwrap().redirects,
        Redirects::Follow
    );
    for wrong in [
        "timeout_seconds = 0",
        "timeout_seconds = -1",
        "timeout_seconds = \"5\"",
        "cookies = 1",
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
        cookies: Cookies::Off,
    };
    write(doc.as_table_mut(), &before, &after);
    assert_eq!(
        doc.to_string(),
        "# keep\nfollow_redirects = false\ntimeout_seconds = 12\ncookies = false\n"
    );
    let on = TransportSettings {
        cookies: Cookies::On,
        ..after
    };
    write(doc.as_table_mut(), &after, &on);
    assert!(doc.to_string().ends_with("cookies = true\n"));
    let after = on;
    write(doc.as_table_mut(), &after, &TransportSettings::default());
    assert_eq!(doc.to_string(), "");
}
