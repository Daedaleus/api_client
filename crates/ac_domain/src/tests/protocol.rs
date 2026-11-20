use super::*;

const STANDARD: [HttpMethod; 7] = [
    HttpMethod::Get,
    HttpMethod::Post,
    HttpMethod::Put,
    HttpMethod::Patch,
    HttpMethod::Delete,
    HttpMethod::Head,
    HttpMethod::Options,
];

#[test]
fn standard_methods_round_trip() {
    for method in STANDARD {
        let parsed: HttpMethod = method.as_str().parse().unwrap();
        assert_eq!(parsed, method);
        assert_eq!(parsed.to_string(), method.as_str());
    }
}

#[test]
fn unknown_token_becomes_custom() {
    let method: HttpMethod = "PROPFIND".parse().unwrap();
    assert_eq!(method, HttpMethod::Custom("PROPFIND".into()));
    assert_eq!(method.to_string(), "PROPFIND");
}

#[test]
fn methods_are_case_sensitive() {
    let method: HttpMethod = "get".parse().unwrap();
    assert_eq!(method, HttpMethod::Custom("get".into()));
}

#[test]
fn status_codes_outside_three_digits_are_rejected() {
    for code in [0, 99, 1000, u16::MAX] {
        let error = StatusCode::new(code).unwrap_err();
        assert_eq!(error, InvalidStatus(code));
        assert!(error.to_string().contains("100–999"));
    }
    assert_eq!(StatusCode::new(100).unwrap().as_u16(), 100);
    assert_eq!(StatusCode::new(999).unwrap().to_string(), "999");
}

#[test]
fn status_class_follows_the_first_digit() {
    let class = |code| StatusCode::new(code).unwrap().class();
    assert_eq!(class(101), StatusClass::Informational);
    assert_eq!(class(204), StatusClass::Success);
    assert_eq!(class(304), StatusClass::Redirection);
    assert_eq!(class(404), StatusClass::ClientError);
    assert_eq!(class(503), StatusClass::ServerError);
    assert_eq!(class(799), StatusClass::Nonstandard);
}

#[test]
fn canonical_reason_for_common_codes() {
    let reason = |code| StatusCode::new(code).unwrap().canonical_reason();
    for code in [
        100, 101, 200, 201, 202, 204, 206, 301, 302, 303, 304, 307, 308, 400, 401, 403, 404, 405,
        406, 408, 409, 410, 413, 415, 422, 429, 500, 501, 502, 503, 504,
    ] {
        assert!(reason(code).is_some(), "{code} has no reason");
    }
    assert_eq!(reason(200), Some("OK"));
    assert_eq!(reason(404), Some("Not Found"));
    assert_eq!(reason(299), None);
}

#[test]
fn invalid_tokens_are_rejected() {
    for name in ["", "GE T", "GET\n", "PÖST", "a(b)"] {
        let error = name.parse::<HttpMethod>().unwrap_err();
        assert_eq!(error, InvalidMethod(name.into()));
        assert!(error.to_string().contains("invalid HTTP method"));
    }
}

#[test]
fn header_names_are_tokens_and_values_have_no_line_breaks() {
    assert!(is_token("X-Request-Id"));
    assert!(!is_token("X Request"));
    assert!(!is_token(""));
    assert!(header::is_valid_value("application/json; charset=utf-8"));
    assert!(header::is_valid_value(""));
    for bad in ["a\r\nInjected: yes", "a\nb", "a\0b"] {
        assert!(!header::is_valid_value(bad), "{bad:?}");
    }
}
