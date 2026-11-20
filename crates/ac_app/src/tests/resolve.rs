use super::*;
use crate::text::keys;

/// Method and URL only.
fn simple(method: &str, url: &str) -> Result<ResolvedRequest, AppError> {
    resolve_request(method, url, &[], &Body::None, &HashMap::new())
}

fn entry(name: &str, value: &str, enabled: bool) -> HeaderEntry {
    HeaderEntry {
        name: name.into(),
        value: value.into(),
        enabled,
    }
}

#[test]
fn complete_input_is_taken_as_is() {
    let request = simple("POST", "https://api.example.com/users?page=2").unwrap();
    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.url.as_str(), "https://api.example.com/users?page=2");
    assert_eq!(request.headers, vec![]);
    assert_eq!(request.body, None);
}

#[test]
fn whitespace_is_trimmed_and_http_assumed() {
    let request = simple(" GET ", "  localhost:8080/health  ").unwrap();
    assert_eq!(request.method, HttpMethod::Get);
    assert_eq!(request.url.as_str(), "http://localhost:8080/health");
}

#[test]
fn enabled_named_headers_are_sent_in_order() {
    let headers = [
        entry("Accept", "application/json", true),
        entry("X-Debug", "1", false),
        entry("  ", "ignored", true),
        entry(" X-Trace ", "a b", true),
    ];
    let request =
        resolve_request("GET", "example.com", &headers, &Body::None, &HashMap::new()).unwrap();
    assert_eq!(
        request.headers,
        [
            Header::new("Accept", "application/json"),
            Header::new("X-Trace", "a b")
        ]
    );
}

#[test]
fn bodies_get_a_content_type_unless_one_is_set() {
    let json = resolve_request(
        "POST",
        "x.example",
        &[],
        &Body::Json("{\"a\":1}".into()),
        &HashMap::new(),
    )
    .unwrap();
    assert_eq!(json.body.as_deref(), Some(b"{\"a\":1}".as_slice()));
    assert_eq!(
        json.headers,
        [Header::new("Content-Type", "application/json")]
    );

    let text = resolve_request(
        "POST",
        "x.example",
        &[],
        &Body::Text("hi".into()),
        &HashMap::new(),
    )
    .unwrap();
    assert_eq!(
        text.headers,
        [Header::new("Content-Type", "text/plain; charset=utf-8")]
    );

    let own = [entry("content-type", "application/vnd.api+json", true)];
    let custom = resolve_request(
        "POST",
        "x.example",
        &own,
        &Body::Json("{}".into()),
        &HashMap::new(),
    )
    .unwrap();
    assert_eq!(
        custom.headers,
        [Header::new("content-type", "application/vnd.api+json")]
    );

    // A disabled Content-Type does not count.
    let disabled = [entry("Content-Type", "text/xml", false)];
    let request = resolve_request(
        "POST",
        "x.example",
        &disabled,
        &Body::Json("{}".into()),
        &HashMap::new(),
    )
    .unwrap();
    assert_eq!(
        request.headers,
        [Header::new("Content-Type", "application/json")]
    );
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
        let error = simple(method, url).unwrap_err();
        assert_eq!(error.user_text().key, key, "{method} {url}");
        assert_ne!(error.to_string(), "");
    }
}

#[test]
fn headers_that_cannot_be_sent_are_refused() {
    for header in [
        entry("Bad Name", "x", true),
        entry("X-Inject", "a\r\nEvil: yes", true),
    ] {
        let error = resolve_request("GET", "x.example", &[header], &Body::None, &HashMap::new())
            .unwrap_err();
        assert_eq!(error.user_text().key, keys::HEADER_INVALID);
        assert!(error.to_string().contains("invalid header"));
    }
    let error = resolve_request(
        "GET",
        "x.example",
        &[entry("Bad Name", "x", true)],
        &Body::None,
        &HashMap::new(),
    )
    .unwrap_err();
    assert_eq!(error.user_text().args["name"], "Bad Name");
}

#[test]
fn variables_are_resolved_in_url_headers_and_body() {
    let variables = HashMap::from([
        ("baseUrl", "https://api.example.com"),
        ("token", "secret"),
        ("header", "X-Token"),
        ("id", "42"),
    ]);
    let request = resolve_request(
        "PUT",
        "{{baseUrl}}/items/{{ id }}",
        &[
            entry("{{header}}", "Bearer {{token}}", true),
            entry("X-Off", "{{missing}}", false),
        ],
        &Body::Json("{\"id\": {{id}}}".into()),
        &variables,
    )
    .unwrap();
    assert_eq!(request.url.as_str(), "https://api.example.com/items/42");
    assert_eq!(request.headers[0], Header::new("X-Token", "Bearer secret"));
    assert_eq!(request.body.as_deref(), Some(b"{\"id\": 42}".as_slice()));
}

#[test]
fn unknown_variables_stop_the_request() {
    for (url, header, body) in [
        ("{{missing}}/x", "a", "b"),
        ("x.example", "{{missing}}", "b"),
        ("x.example", "a", "{{missing}}"),
    ] {
        let error = resolve_request(
            "POST",
            url,
            &[entry("X-A", header, true)],
            &Body::Text(body.into()),
            &HashMap::new(),
        )
        .unwrap_err();
        assert_eq!(error.user_text().key, keys::VARIABLE_UNKNOWN);
        assert_eq!(error.user_text().args["name"], "missing");
    }
}

#[test]
fn errors_carry_the_offending_value() {
    let text = simple("GE T", "x").unwrap_err().user_text();
    assert_eq!(text.args["method"], "GE T");
    let text = simple("GET", "ftp://x").unwrap_err().user_text();
    assert_eq!(text.args["scheme"], "ftp");
    let text = simple("GET", "http://exa mple.com")
        .unwrap_err()
        .user_text();
    assert_ne!(text.args["reason"], "");
}
