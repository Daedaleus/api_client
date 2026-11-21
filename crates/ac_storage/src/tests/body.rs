use toml_edit::DocumentMut;

use super::*;

fn parse(text: &str) -> Result<Body, StorageError> {
    let doc: DocumentMut = text.parse().unwrap();
    read(doc.get("body"), Path::new("x.toml"))
}

fn round_trip(body: &Body) -> Body {
    let mut doc = DocumentMut::new();
    if let Some(item) = item(body) {
        doc.insert("body", item);
    }
    parse(&doc.to_string()).unwrap()
}

#[test]
fn every_body_round_trips() {
    for body in [
        Body::None,
        Body::Json("{\"a\": 1}".into()),
        Body::Text("line 1\nline 2".into()),
        Body::Form(vec![
            FormField {
                name: "q".into(),
                value: "tea".into(),
                enabled: true,
            },
            FormField {
                name: "off".into(),
                value: String::new(),
                enabled: false,
            },
        ]),
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Teapot".into(),
                enabled: true,
                file: false,
            },
            MultipartField {
                name: "avatar".into(),
                value: "files/avatar.png".into(),
                enabled: true,
                file: true,
            },
        ]),
        Body::Binary("data/payload.bin".into()),
    ] {
        assert_eq!(round_trip(&body), body);
    }
}

#[test]
fn files_are_written_readably() {
    let mut doc = DocumentMut::new();
    let body = Body::Multipart(vec![MultipartField {
        name: "avatar".into(),
        value: "files/avatar.png".into(),
        enabled: false,
        file: true,
    }]);
    doc.insert("body", item(&body).unwrap());
    let text = doc.to_string();
    assert!(text.starts_with("[body]\ntype = \"multipart\""), "{text}");
    assert!(
        text.contains("file = \"files/avatar.png\"\nenabled = false"),
        "{text}"
    );
}

#[test]
fn inline_fields_and_defaults_are_read_and_mistakes_reported() {
    assert_eq!(
        parse("body = { type = \"form\", fields = [{ name = \"a\" }] }").unwrap(),
        Body::Form(vec![FormField {
            name: "a".into(),
            value: String::new(),
            enabled: true,
        }])
    );
    assert_eq!(
        parse("body = { type = \"form\" }").unwrap(),
        Body::Form(Vec::new())
    );
    for text in [
        "body = 1",
        "body = { type = \"xml\" }",
        "body = { type = \"form\", fields = 3 }",
        "body = { type = \"form\", fields = [1] }",
        "body = { type = \"multipart\", fields = [{ value = \"no name\" }] }",
    ] {
        assert!(parse(text).is_err(), "{text}");
    }
}
