use super::*;

#[test]
fn json_is_reindented_keeping_numbers_order_and_strings() {
    let body = br#"{"b":1.10,"a":[1, 2],"s":"x, {y}: \"z\"","e":{},"l":[ ]}"#;
    let view = preview(Some("application/json; charset=utf-8"), body);
    assert!(view.pretty);
    assert_eq!(
        view.text,
        "{\n  \"b\": 1.10,\n  \"a\": [\n    1,\n    2\n  ],\n  \"s\": \"x, {y}: \\\"z\\\"\",\n  \"e\": {},\n  \"l\": []\n}"
    );
}

#[test]
fn json_variants_are_recognised() {
    assert!(preview(Some("application/problem+json"), b"[1]").pretty);
    assert!(preview(Some("Application/JSON"), b"true").pretty);
    assert!(!preview(Some("text/plain"), b"[1]").pretty);
    assert!(!preview(None, b"[1]").pretty);
}

#[test]
fn invalid_json_is_shown_as_text() {
    let view = preview(Some("application/json"), b"{oops");
    assert!(!view.pretty);
    assert_eq!(view.text, "{oops");
}

#[test]
fn binary_bodies_have_no_text() {
    let view = preview(Some("image/png"), &[0x89, b'P', b'N', b'G', 0xff]);
    assert!(view.binary);
    assert_eq!(view.text, "");
}

#[test]
fn large_bodies_are_truncated_on_a_char_boundary() {
    let view = preview_with_limit(Some("text/plain"), "aäb".as_bytes(), 2);
    assert!(view.truncated);
    assert_eq!(view.text, "a");

    let view = preview_with_limit(Some("application/json"), b"[1,2,3]", 4);
    assert!(view.truncated);
    assert!(!view.pretty);
    assert_eq!(view.text, "[1,2");

    let view = preview(Some("text/plain"), b"short");
    assert!(!view.truncated);
    assert_eq!(view.text, "short");
}

#[test]
fn serializes_in_camel_case() {
    let json = serde_json::to_value(preview(None, b"x")).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "text": "x", "pretty": false, "truncated": false, "binary": false })
    );
}
