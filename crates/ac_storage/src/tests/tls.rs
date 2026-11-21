use toml_edit::DocumentMut;

use super::*;

fn read_text(text: &str) -> Result<CertificateCheck, StorageError> {
    let doc: DocumentMut = text.parse().unwrap();
    read(doc.as_table(), Path::new("x.toml"))
}

#[test]
fn every_setting_round_trips_and_unchanged_ones_keep_the_file() {
    for check in [
        CertificateCheck::Inherit,
        CertificateCheck::On,
        CertificateCheck::Off,
    ] {
        let mut doc: DocumentMut = "# keep me\nname = \"x\"\nverify_tls = false\n"
            .parse()
            .unwrap();
        write(doc.as_table_mut(), CertificateCheck::Off, check);
        let text = doc.to_string();
        assert!(text.starts_with("# keep me"), "{text}");
        assert_eq!(read_text(&text).unwrap(), check);
    }
    assert_eq!(read_text("").unwrap(), CertificateCheck::Inherit);
    assert!(read_text("verify_tls = \"no\"").is_err());
}
