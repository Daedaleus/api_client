use std::path::PathBuf;

use super::*;
use crate::command::{Command, Query};
use crate::settings::{AppSettings, NetworkSettings, ProxySetting};
use crate::test_support::Fixture;
use crate::text::keys;
use crate::view::ProxyView;

fn manual(url: &str) -> SettingsView {
    SettingsView {
        proxy: ProxyView::Manual {
            url: url.into(),
            no_proxy: "localhost".into(),
        },
        ca_certificates: vec!["C:/certs/corp.pem".into()],
        client_certificates: Vec::new(),
    }
}

#[tokio::test]
async fn settings_are_applied_before_they_are_saved() {
    let fixture = Fixture::new();
    let QueryResult::Settings { settings, problem } = fixture.app.query(Query::Settings).unwrap()
    else {
        panic!("expected settings");
    };
    assert_eq!(settings, SettingsView::default());
    assert_eq!(problem, None);

    assert_eq!(
        fixture.run(Command::SaveSettings {
            settings: manual("http://proxy:8080"),
        }),
        CommandAck::SettingsSaved
    );
    let saved = fixture.settings.saved().unwrap();
    assert!(
        matches!(&saved.network.proxy, ProxySetting::Manual { url, .. } if url == "http://proxy:8080")
    );
    assert_eq!(fixture.http.network(), Some(saved.network.clone()));

    // Settings that cannot work are neither applied nor saved.
    let error = fixture.fail(Command::SaveSettings {
        settings: manual("proxy:8080"),
    });
    assert_eq!(error.user_text().key, keys::SETTINGS);
    assert_eq!(fixture.settings.saved(), Some(saved));
}

#[tokio::test]
async fn saved_settings_are_applied_at_start_and_problems_shown() {
    let fixture = Fixture::new();
    fixture
        .settings
        .save(&AppSettings {
            network: NetworkSettings {
                proxy: ProxySetting::None,
                ca_certificates: vec![PathBuf::from("ca.pem")],
                client_certificates: Vec::new(),
            },
        })
        .unwrap();
    fixture.app.apply_saved_settings().unwrap();
    assert_eq!(fixture.http.network().unwrap().proxy, ProxySetting::None);

    fixture
        .settings
        .save(&manual("nonsense").to_settings())
        .unwrap();
    assert!(fixture.app.apply_saved_settings().is_err());
    let QueryResult::Settings { problem, .. } = fixture.app.query(Query::Settings).unwrap() else {
        panic!("expected settings");
    };
    assert_eq!(problem.unwrap().key, keys::SETTINGS);
    // Saving working settings clears the problem.
    fixture.run(Command::SaveSettings {
        settings: SettingsView::default(),
    });
    let QueryResult::Settings { problem, .. } = fixture.app.query(Query::Settings).unwrap() else {
        panic!("expected settings");
    };
    assert_eq!(problem, None);

    fixture.settings.failing("disk gone");
    assert!(fixture.app.query(Query::Settings).is_err());
    assert!(fixture.app.apply_saved_settings().is_err());
}

#[tokio::test]
async fn passphrases_are_stored_encrypted_and_applied_decrypted() {
    use base64::Engine as _;

    use crate::ports::SecretError;
    use crate::view::ClientCertificateView;

    let fixture = Fixture::new();
    let with = |new_passphrase: Option<&str>, encrypted: Option<String>| SettingsView {
        client_certificates: vec![ClientCertificateView {
            host: "api.internal".into(),
            certificate: "me.p12".into(),
            key: String::new(),
            encrypted_passphrase: encrypted,
            new_passphrase: new_passphrase.map(Into::into),
        }],
        ..SettingsView::default()
    };
    fixture.run(Command::SaveSettings {
        settings: with(Some("hunter2"), None),
    });
    let saved = fixture.settings.saved().unwrap();
    let encrypted = saved.network.client_certificates[0]
        .passphrase
        .clone()
        .unwrap();
    assert!(!encrypted.0.contains("hunter2"));
    // Encrypted for the user's own key only.
    let sealed = base64::engine::general_purpose::STANDARD
        .decode(&encrypted.0)
        .unwrap();
    assert!(String::from_utf8_lossy(&sealed).contains(crate::testing::FakeVault::OWN_KEY));
    assert_eq!(
        fixture.http.identities()[0].passphrase.as_deref(),
        Some("hunter2")
    );
    // The dialog sees only the encrypted value.
    let QueryResult::Settings { settings, .. } = fixture.app.query(Query::Settings).unwrap() else {
        panic!("expected settings");
    };
    assert_eq!(
        settings.client_certificates[0]
            .encrypted_passphrase
            .as_deref(),
        Some(encrypted.0.as_str())
    );
    assert_eq!(settings.client_certificates[0].new_passphrase, None);

    // Kept when not changed, applied at start, removed with "".
    fixture.run(Command::SaveSettings {
        settings: with(None, Some(encrypted.0.clone())),
    });
    assert_eq!(
        fixture
            .settings
            .saved()
            .unwrap()
            .network
            .client_certificates[0]
            .passphrase,
        Some(encrypted.clone())
    );
    fixture.app.apply_saved_settings().unwrap();
    assert_eq!(
        fixture.http.identities()[0].passphrase.as_deref(),
        Some("hunter2")
    );
    fixture.run(Command::SaveSettings {
        settings: with(Some(""), Some(encrypted.0.clone())),
    });
    assert_eq!(
        fixture
            .settings
            .saved()
            .unwrap()
            .network
            .client_certificates[0]
            .passphrase,
        None
    );
    assert_eq!(fixture.http.identities()[0].passphrase, None);

    // Damaged or undecryptable passphrases are reported, nothing is saved.
    for damaged in [
        "not base64!".to_owned(),
        base64::engine::general_purpose::STANDARD.encode("{}"),
    ] {
        let error = fixture.fail(Command::SaveSettings {
            settings: with(None, Some(damaged)),
        });
        assert_eq!(error.user_text().key, keys::SECRET_DAMAGED);
    }
    fixture
        .vault
        .fail_next(SecretError::Key("no key file".into()));
    let error = fixture.fail(Command::SaveSettings {
        settings: with(Some("new"), None),
    });
    assert_eq!(error.user_text().key, keys::SECRET_KEY);
    let error = fixture.fail(Command::SaveSettings {
        settings: SettingsView {
            client_certificates: vec![ClientCertificateView {
                host: "x".into(),
                ..ClientCertificateView::default()
            }],
            ..SettingsView::default()
        },
    });
    assert_eq!(error.user_text().key, keys::SETTINGS);
}
