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
