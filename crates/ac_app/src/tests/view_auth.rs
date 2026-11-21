use super::*;

#[test]
fn every_kind_round_trips_and_matches_the_ui_contract() {
    let all = [
        Auth::Inherit,
        Auth::None,
        Auth::Basic {
            username: "alice".into(),
            password: "{{password}}".into(),
        },
        Auth::Bearer {
            token: "{{token}}".into(),
        },
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "k".into(),
            placement: KeyPlacement::Header,
        },
        Auth::ApiKey {
            name: "key".into(),
            value: "k".into(),
            placement: KeyPlacement::Query,
        },
        Auth::OAuth2(OAuth2 {
            token_url: "https://auth.example.com/token".into(),
            client_id: "app".into(),
            client_secret: "{{secret}}".into(),
            scope: "read write".into(),
            ..Default::default()
        }),
        Auth::OAuth2(OAuth2 {
            grant: ac_domain::auth::OAuth2Grant::AuthorizationCode,
            token_url: "https://auth.example.com/token".into(),
            auth_url: "https://auth.example.com/authorize".into(),
            redirect_url: "http://127.0.0.1:8080/cb".into(),
            client_id: "app".into(),
            ..Default::default()
        }),
    ];
    for auth in &all {
        assert_eq!(&AuthDraft::from_auth(auth).to_auth(), auth);
    }
    let json = serde_json::to_value(AuthDraft::from_auth(&all[5])).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "kind": "apiKey", "name": "key", "value": "k", "placement": "query" })
    );
    let json = serde_json::to_value(AuthDraft::from_auth(&all[6])).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "kind": "oauth2", "grant": "clientCredentials",
            "tokenUrl": "https://auth.example.com/token", "authUrl": "", "redirectUrl": "",
            "clientId": "app", "clientSecret": "{{secret}}", "scope": "read write" })
    );
    let json = serde_json::to_value(AuthDraft::from_auth(&all[7])).unwrap();
    assert_eq!(json["grant"], "authorizationCode");
    assert_eq!(json["redirectUrl"], "http://127.0.0.1:8080/cb");
    let draft: AuthDraft =
        serde_json::from_value(serde_json::json!({ "kind": "inherit" })).unwrap();
    assert_eq!(draft, AuthDraft::default());
}

#[test]
fn certificate_checks_round_trip_and_match_the_ui_contract() {
    for check in [
        CertificateCheck::Inherit,
        CertificateCheck::On,
        CertificateCheck::Off,
    ] {
        assert_eq!(CertificateCheckView::from_check(check).to_check(), check);
    }
    assert_eq!(
        serde_json::to_value(CertificateCheckView::Off).unwrap(),
        serde_json::json!("off")
    );
}
