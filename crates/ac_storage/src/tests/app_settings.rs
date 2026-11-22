use super::*;

fn manual() -> AppSettings {
    AppSettings {
        network: NetworkSettings {
            proxy: ProxySetting::Manual {
                url: "http://proxy:8080".into(),
                no_proxy: "localhost,.internal".into(),
            },
            ca_certificates: vec![PathBuf::from("C:/certs/ca.pem")],
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
