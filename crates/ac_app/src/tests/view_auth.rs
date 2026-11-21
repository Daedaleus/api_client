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
    ];
    for auth in &all {
        assert_eq!(&AuthDraft::from_auth(auth).to_auth(), auth);
    }
    let json = serde_json::to_value(AuthDraft::from_auth(&all[5])).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "kind": "apiKey", "name": "key", "value": "k", "placement": "query" })
    );
    let draft: AuthDraft =
        serde_json::from_value(serde_json::json!({ "kind": "inherit" })).unwrap();
    assert_eq!(draft, AuthDraft::default());
}
