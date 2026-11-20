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
fn invalid_tokens_are_rejected() {
    for name in ["", "GE T", "GET\n", "PÖST", "a(b)"] {
        let error = name.parse::<HttpMethod>().unwrap_err();
        assert_eq!(error, InvalidMethod(name.into()));
        assert!(error.to_string().contains("invalid HTTP method"));
    }
}
