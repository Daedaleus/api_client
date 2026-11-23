use url::Url;

use super::*;

/// 2026-01-01 00:00:00 UTC.
const NOW: UnixSeconds = 1_767_225_600;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn jar_with(from: &str, headers: &[&str]) -> CookieJar {
    let mut jar = CookieJar::new();
    for header in headers {
        jar.store(&url(from), header, NOW);
    }
    jar
}

fn header(jar: &CookieJar, to: &str) -> Option<String> {
    jar.header_for(&url(to), NOW)
}

#[test]
fn a_session_cookie_goes_back_to_the_same_host() {
    let jar = jar_with("https://api.example.com/login", &["session=abc; HttpOnly"]);
    assert_eq!(
        header(&jar, "https://api.example.com/users"),
        Some("session=abc".into())
    );
    // Host-only: not to other hosts or subdomains.
    assert_eq!(header(&jar, "https://example.com/"), None);
    assert_eq!(header(&jar, "https://www.api.example.com/"), None);
    let cookie = jar.cookies(NOW).next().unwrap();
    assert_eq!(
        cookie,
        &Cookie {
            name: "session".into(),
            value: "abc".into(),
            domain: "api.example.com".into(),
            host_only: true,
            path: "/".into(),
            expires: None,
            secure: false,
            http_only: true,
        }
    );
}

#[test]
fn domain_cookies_reach_subdomains_but_not_other_sites() {
    let jar = jar_with(
        "https://shop.example.com/",
        &[
            "a=1; Domain=.Example.com",
            "b=2; Domain=example.com; Path=/api",
            // Neither another site nor a bare top-level domain.
            "evil=1; Domain=other.com",
            "tld=1; Domain=com",
        ],
    );
    assert_eq!(
        header(&jar, "https://www.example.com/api/items"),
        Some("b=2; a=1".into())
    );
    assert_eq!(header(&jar, "https://example.com/"), Some("a=1".into()));
    assert_eq!(header(&jar, "https://notexample.com/"), None);
    assert_eq!(jar.cookies(NOW).count(), 2);
}

#[test]
fn paths_match_like_in_a_browser() {
    let jar = jar_with(
        "https://x.test/docs/guide/intro",
        &["default=1", "api=2; Path=/api", "bad=3; Path=relative"],
    );
    // Without a path attribute: the folder of the request.
    assert_eq!(
        header(&jar, "https://x.test/docs/guide"),
        Some("default=1; bad=3".into())
    );
    assert_eq!(
        header(&jar, "https://x.test/docs/guide/x"),
        Some("default=1; bad=3".into())
    );
    assert_eq!(header(&jar, "https://x.test/docs/guidebook"), None);
    assert_eq!(header(&jar, "https://x.test/api"), Some("api=2".into()));
    assert_eq!(header(&jar, "https://x.test/api/v1"), Some("api=2".into()));
    assert_eq!(header(&jar, "https://x.test/apix"), None);

    let root = jar_with("https://x.test/", &["r=1"]);
    assert_eq!(
        header(&root, "https://x.test/any/where"),
        Some("r=1".into())
    );
    let top = jar_with("https://x.test/login", &["t=1; Path=/api/"]);
    assert_eq!(header(&top, "https://x.test/api/x"), Some("t=1".into()));
}

#[test]
fn expiry_max_age_and_removal() {
    let mut jar = jar_with(
        "https://x.test/",
        &[
            "kept=1; Expires=Wed, 01 Jan 2031 00:00:00 GMT",
            "gone=1; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
            "short=1; Max-Age=60; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
            "bad=1; Max-Age=soon",
        ],
    );
    assert_eq!(
        header(&jar, "https://x.test/"),
        Some("kept=1; short=1; bad=1".into())
    );
    assert_eq!(
        jar.header_for(&url("https://x.test/"), NOW + 61),
        Some("kept=1; bad=1".into())
    );
    // An expired cookie removes the one it names; replacing keeps one.
    jar.store(&url("https://x.test/"), "kept=2", NOW);
    jar.store(&url("https://x.test/"), "bad=1; Max-Age=0", NOW);
    assert_eq!(
        header(&jar, "https://x.test/"),
        Some("kept=2; short=1".into())
    );
    assert_eq!(jar.cookies(NOW + 61).count(), 1);
}

#[test]
fn secure_cookies_need_a_secure_connection() {
    let jar = jar_with("https://x.test/", &["s=1; Secure"]);
    assert_eq!(header(&jar, "https://x.test/"), Some("s=1".into()));
    assert_eq!(header(&jar, "http://x.test/"), None);
    // Not settable over plain HTTP …
    assert_eq!(
        jar_with("http://x.test/", &["s=1; Secure"])
            .cookies(NOW)
            .count(),
        0
    );
    // … except on the own machine, which counts as secure.
    let local = jar_with("http://localhost:8080/", &["s=1; Secure"]);
    assert_eq!(
        header(&local, "http://localhost:8080/x"),
        Some("s=1".into())
    );
    let ip = jar_with("http://127.0.0.1:9/", &["s=1; Secure; Domain=127.0.0.1"]);
    assert_eq!(header(&ip, "http://127.0.0.1:9/"), Some("s=1".into()));
}

#[test]
fn invalid_headers_are_ignored() {
    let jar = jar_with(
        "https://x.test/",
        &["", "novalue", "=nameless", " spaced = v ; path=/"],
    );
    assert_eq!(header(&jar, "https://x.test/"), Some("spaced=v".into()));
    let mut jar = CookieJar::new();
    jar.store(&url("data:text/plain,x"), "a=1", NOW);
    assert_eq!(jar.cookies(NOW).count(), 0);
    let ip = jar_with("http://10.0.0.1/", &["a=1; Domain=0.0.1"]);
    assert_eq!(ip.cookies(NOW).count(), 0);
}

#[test]
fn cookies_can_be_edited_removed_and_cleared() {
    let mut jar = jar_with("https://x.test/", &["a=1", "b=2"]);
    let mut edited = jar.cookies(NOW).next().unwrap().clone();
    edited.value = "changed".into();
    jar.set(edited, NOW);
    assert_eq!(
        header(&jar, "https://x.test/"),
        Some("a=changed; b=2".into())
    );
    assert!(jar.remove("x.test", "/", "a"));
    assert!(!jar.remove("x.test", "/", "a"));
    assert_eq!(header(&jar, "https://x.test/"), Some("b=2".into()));
    jar.clear();
    assert_eq!(header(&jar, "https://x.test/"), None);
}

#[test]
fn cookie_dates_are_read_leniently() {
    let expected = Some(784_111_777);
    assert_eq!(parse_cookie_date("Sun, 06 Nov 1994 08:49:37 GMT"), expected);
    assert_eq!(
        parse_cookie_date("Sunday, 06-Nov-94 08:49:37 GMT"),
        expected
    );
    assert_eq!(parse_cookie_date("Sun Nov  6 08:49:37 1994"), expected);
    assert_eq!(parse_cookie_date("6 november 1994 8:49:37"), expected);
    assert_eq!(parse_cookie_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
    assert_eq!(
        parse_cookie_date("Sat, 29 Feb 2020 12:00:00 GMT"),
        Some(1_582_977_600)
    );
    assert_eq!(
        parse_cookie_date("Mon, 01 Jan 2035 00:00:00 GMT"),
        Some(2_051_222_400)
    );
    for invalid in [
        "",
        "tomorrow",
        "Sun, 06 Nov 1994",
        "Sun, 32 Nov 1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 24:00:00 GMT",
        "Sun, 06 Nov 1500 08:49:37 GMT",
        "Sun, 06 Xyz 1994 08:49:37 GMT",
    ] {
        assert_eq!(parse_cookie_date(invalid), None, "{invalid}");
    }
}
