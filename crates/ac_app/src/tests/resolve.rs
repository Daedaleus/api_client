use super::*;
use crate::text::keys;

#[test]
fn complete_input_is_taken_as_is() {
    let request = resolve_request("POST", "https://api.example.com/users?page=2").unwrap();
    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.url.as_str(), "https://api.example.com/users?page=2");
    assert_eq!(request.headers, vec![]);
}

#[test]
fn whitespace_is_trimmed_and_http_assumed() {
    let request = resolve_request(" GET ", "  localhost:8080/health  ").unwrap();
    assert_eq!(request.method, HttpMethod::Get);
    assert_eq!(request.url.as_str(), "http://localhost:8080/health");
}

#[test]
fn invalid_input_is_reported_with_a_user_text() {
    let cases = [
        ("GE T", "example.com", keys::METHOD_INVALID),
        ("GET", "   ", keys::URL_EMPTY),
        ("GET", "http://exa mple.com", keys::URL_INVALID),
        ("GET", "ftp://example.com/file", keys::URL_SCHEME),
    ];
    for (method, url, key) in cases {
        let error = resolve_request(method, url).unwrap_err();
        assert_eq!(error.user_text().key, key, "{method} {url}");
        assert_ne!(error.to_string(), "");
    }
}

#[test]
fn errors_carry_the_offending_value() {
    let text = resolve_request("GE T", "x").unwrap_err().user_text();
    assert_eq!(text.args["method"], "GE T");
    let text = resolve_request("GET", "ftp://x").unwrap_err().user_text();
    assert_eq!(text.args["scheme"], "ftp");
    let text = resolve_request("GET", "http://exa mple.com")
        .unwrap_err()
        .user_text();
    assert_ne!(text.args["reason"], "");
}
