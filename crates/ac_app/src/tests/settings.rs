use url::Url;

use super::*;

fn matches(pattern: &str, url: &str) -> bool {
    HostPattern::new(pattern).matches(&Url::parse(url).unwrap())
}

#[test]
fn host_patterns_match_hosts_ports_and_subdomains() {
    assert!(matches(" API.example.com ", "https://api.example.com/x"));
    assert!(!matches("api.example.com", "https://other.example.com/"));
    assert!(matches(
        "api.example.com:8443",
        "https://api.example.com:8443/"
    ));
    assert!(!matches("api.example.com:8443", "https://api.example.com/"));
    assert!(matches("api.example.com:443", "https://api.example.com/"));
    assert!(matches("*.internal", "https://a.b.internal/"));
    assert!(!matches("*.internal", "https://internal/"));
    assert!(!matches("*.internal", "https://xinternal/"));
    assert!(matches("[::1]:8443", "https://[::1]:8443/"));
    assert!(matches("[::1]", "https://[::1]/"));
    assert!(matches("127.0.0.1", "https://127.0.0.1:9/"));
    assert!(!matches("", "https://a/"));
    assert!(!matches("a", "data:text/plain,x"));
    // Not a port: the whole pattern is the host name.
    assert!(!matches("host:notaport", "https://host/"));
    assert_eq!(HostPattern::new(" A.b ").as_str(), "a.b");
}

#[test]
fn identities_keep_the_passphrase_out_of_debug_output() {
    let identity = ClientIdentity {
        host: HostPattern::new("a"),
        certificate: "me.p12".into(),
        key: None,
        passphrase: Some("hunter2".into()),
    };
    let debug = format!("{identity:?}");
    assert!(debug.contains("me.p12") && debug.contains("***"), "{debug}");
    assert!(!debug.contains("hunter2"));
}
