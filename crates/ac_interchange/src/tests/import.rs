use super::*;

#[test]
fn yaml_is_read_into_json_values() {
    let value = parse("a: 1\nb: 1.5\nc: [x, true, null]\n200:\n  ok: yes\n").unwrap();
    assert_eq!(
        value,
        serde_json::json!({"a": 1, "b": 1.5, "c": ["x", true, null], "200": {"ok": "yes"}})
    );
    assert_eq!(parse("{\"a\": 1}").unwrap(), serde_json::json!({"a": 1}));
    assert_eq!(parse("x: .inf").unwrap(), serde_json::json!({"x": ".inf"}));
}

#[test]
fn unreadable_and_unknown_texts_are_reported() {
    assert!(matches!(parse("a: [1"), Err(ImportError::Parse(_))));
    assert!(matches!(parse(""), Err(ImportError::Parse(_))));
    assert_eq!(import_collection("a: 1"), Err(ImportError::Unknown));
    assert_eq!(
        import_collection("{\"swagger\": \"2.0\"}"),
        Err(ImportError::Swagger2)
    );
    assert!(ImportError::Swagger2.to_string().contains("OpenAPI 3"));
}
