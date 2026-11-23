use super::*;

const fn settings(redirects: Redirects, timeout_seconds: Option<u32>) -> TransportSettings {
    TransportSettings {
        redirects,
        timeout_seconds,
        cookies: Cookies::Inherit,
    }
}

#[test]
fn nothing_set_means_the_defaults() {
    assert_eq!(effective([]), EffectiveTransport::default());
    let inherit = TransportSettings::default();
    let result = effective([&inherit, &inherit]);
    assert!(result.follow_redirects);
    assert_eq!(result.timeout, Duration::from_secs(30));
    assert!(result.cookies);
}

#[test]
fn cookies_are_switched_off_by_the_innermost_setting() {
    let off = TransportSettings {
        cookies: Cookies::Off,
        ..TransportSettings::default()
    };
    let on = TransportSettings {
        cookies: Cookies::On,
        ..TransportSettings::default()
    };
    assert!(!effective([&TransportSettings::default(), &off]).cookies);
    assert!(effective([&on, &off]).cookies);
}

#[test]
fn the_innermost_setting_wins_for_each_setting() {
    let request = settings(Redirects::Inherit, Some(5));
    let folder = settings(Redirects::Stop, Some(60));
    let collection = settings(Redirects::Follow, None);
    let result = effective([&request, &folder, &collection]);
    assert!(!result.follow_redirects);
    assert_eq!(result.timeout, Duration::from_secs(5));

    let result = effective([&TransportSettings::default(), &collection]);
    assert!(result.follow_redirects);
    assert_eq!(result.timeout, Duration::from_secs(30));
}
