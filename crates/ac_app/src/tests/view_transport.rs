use super::*;

#[test]
fn settings_round_trip_and_match_the_ui_contract() {
    for redirects in [Redirects::Inherit, Redirects::Follow, Redirects::Stop] {
        let settings = TransportSettings {
            redirects,
            timeout_seconds: Some(9),
        };
        assert_eq!(
            TransportDraft::from_settings(&settings).to_settings(),
            settings
        );
    }
    let draft = TransportDraft {
        redirects: RedirectsView::Stop,
        timeout_seconds: Some(0),
    };
    assert_eq!(draft.to_settings().timeout_seconds, None);
    assert_eq!(
        serde_json::to_value(draft).unwrap(),
        serde_json::json!({ "redirects": "stop", "timeoutSeconds": 0 })
    );
    let missing: TransportDraft = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, TransportDraft::default());
}
