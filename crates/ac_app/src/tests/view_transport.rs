use super::*;

#[test]
fn settings_round_trip_and_match_the_ui_contract() {
    let cookies = [Cookies::Inherit, Cookies::On, Cookies::Off];
    for (redirects, cookies) in [Redirects::Inherit, Redirects::Follow, Redirects::Stop]
        .into_iter()
        .zip(cookies)
    {
        let settings = TransportSettings {
            redirects,
            timeout_seconds: Some(9),
            cookies,
        };
        assert_eq!(
            TransportDraft::from_settings(&settings).to_settings(),
            settings
        );
    }
    let draft = TransportDraft {
        redirects: RedirectsView::Stop,
        timeout_seconds: Some(0),
        cookies: CookiesView::Off,
    };
    assert_eq!(draft.to_settings().timeout_seconds, None);
    assert_eq!(
        serde_json::to_value(draft).unwrap(),
        serde_json::json!({ "redirects": "stop", "timeoutSeconds": 0, "cookies": "off" })
    );
    let missing: TransportDraft = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, TransportDraft::default());
}
