use super::*;

fn manual() -> AppSettings {
    AppSettings {
        network: NetworkSettings {
            proxy: ProxySetting::Manual {
                url: "http://proxy:8080".into(),
                no_proxy: "localhost,.internal".into(),
            },
            ca_certificates: vec![PathBuf::from("C:/certs/ca.pem")],
            client_certificates: vec![
                ClientCertificate {
                    host: HostPattern::new("*.internal"),
                    certificate: PathBuf::from("C:/certs/me.pem"),
                    key: Some(PathBuf::from("C:/certs/me.key")),
                    passphrase: Some(EncryptedPassphrase("c2VhbGVk".into())),
                },
                ClientCertificate {
                    host: HostPattern::new("api.example.com:8443"),
                    certificate: PathBuf::from("C:/certs/me.p12"),
                    key: None,
                    passphrase: None,
                },
            ],
        },
    }
}

#[test]
fn settings_round_trip_and_default_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let store = FileSettings::new(&temp.path().join("config"));
    assert_eq!(store.load().unwrap(), AppSettings::default());
    store.save(&manual()).unwrap();
    assert_eq!(store.load().unwrap(), manual());
    let text = std::fs::read_to_string(temp.path().join("config").join(SETTINGS_FILE)).unwrap();
    assert!(
        text.starts_with("format = \"api-client/settings\"\nversion = 1\n"),
        "{text}"
    );

    for proxy in [ProxySetting::System, ProxySetting::None] {
        let settings = AppSettings {
            network: NetworkSettings {
                proxy,
                ca_certificates: Vec::new(),
                client_certificates: Vec::new(),
            },
        };
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
    }
}

#[test]
fn damaged_files_are_reported_and_kept() {
    let temp = tempfile::tempdir().unwrap();
    let store = FileSettings::new(temp.path());
    let file = temp.path().join(SETTINGS_FILE);
    for text in [
        "not toml [",
        "format = \"api-client/settings\"\nversion = 1\n[proxy]\nmode = \"carrier pigeon\"\n",
        "format = \"api-client/settings\"\nversion = 1\n[proxy]\nmode = 3\n",
        "format = \"api-client/settings\"\nversion = 1\n[tls]\nca_certificates = \"a.pem\"\n",
        "format = \"api-client/settings\"\nversion = 1\n[tls]\nca_certificates = [1]\n",
    ] {
        std::fs::write(&file, text).unwrap();
        assert!(store.load().is_err(), "{text}");
    }
    // Saving keeps the damaged file as a backup.
    store.save(&manual()).unwrap();
    assert_eq!(store.load().unwrap(), manual());
    let backups = std::fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".bak"))
        .count();
    assert_eq!(backups, 1);
}

#[test]
fn unreadable_places_are_errors() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join(SETTINGS_FILE)).unwrap();
    let store = FileSettings::new(temp.path());
    assert!(store.load().is_err());
}

#[test]
fn client_certificates_are_tables_and_checked() {
    let temp = tempfile::tempdir().unwrap();
    let store = FileSettings::new(temp.path());
    store.save(&manual()).unwrap();
    let text = std::fs::read_to_string(temp.path().join(SETTINGS_FILE)).unwrap();
    assert!(text.contains("[[tls.client_certificates]]"), "{text}");
    assert!(text.contains("passphrase = \"c2VhbGVk\""), "{text}");
    for wrong in [
        "[tls]\nclient_certificates = 1\n",
        "[[tls.client_certificates]]\nhost = 1\n",
    ] {
        std::fs::write(
            temp.path().join(SETTINGS_FILE),
            format!("format = \"api-client/settings\"\nversion = 1\n{wrong}"),
        )
        .unwrap();
        assert!(store.load().is_err(), "{wrong}");
    }
}
