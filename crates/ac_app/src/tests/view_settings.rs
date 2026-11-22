use super::*;

#[test]
fn settings_round_trip_and_match_the_ui_contract() {
    for proxy in [
        ProxySetting::System,
        ProxySetting::None,
        ProxySetting::Manual {
            url: "http://proxy:8080".into(),
            no_proxy: "localhost".into(),
        },
    ] {
        let settings = AppSettings {
            network: NetworkSettings {
                proxy,
                ca_certificates: vec![PathBuf::from("C:/certs/corp.pem")],
            },
        };
        assert_eq!(
            SettingsView::from_settings(&settings).to_settings(),
            settings
        );
    }
    let view = SettingsView {
        proxy: ProxyView::Manual {
            url: " http://p:1 ".into(),
            no_proxy: String::new(),
        },
        ca_certificates: vec!["  ".into(), " a.pem ".into()],
    };
    let settings = view.to_settings();
    assert_eq!(settings.network.ca_certificates, [PathBuf::from("a.pem")]);
    assert!(
        matches!(settings.network.proxy, ProxySetting::Manual { url, .. } if url == "http://p:1")
    );
    assert_eq!(
        serde_json::to_value(&view).unwrap(),
        serde_json::json!({
            "proxy": { "mode": "manual", "url": " http://p:1 ", "noProxy": "" },
            "caCertificates": ["  ", " a.pem "]
        })
    );
}
