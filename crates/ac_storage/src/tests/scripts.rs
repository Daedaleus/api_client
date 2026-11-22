use toml_edit::DocumentMut;

use super::*;

fn round_trip(scripts: &Scripts) -> (Scripts, String) {
    let mut doc = DocumentMut::new();
    write(doc.as_table_mut(), &Scripts::default(), scripts);
    let text = doc.to_string();
    let parsed: DocumentMut = text.parse().unwrap();
    (
        read(parsed.get(KEY), Path::new("x.toml"), "request.scripts").unwrap(),
        text,
    )
}

#[test]
fn scripts_round_trip_as_readable_multi_line_strings() {
    let scripts = Scripts {
        pre_request: "req.setHeader(\"X\", 'y');\nlog(`a\\b ${1}`);\n".into(),
        post_response: String::new(),
    };
    let (back, text) = round_trip(&scripts);
    assert_eq!(back, scripts);
    assert!(text.contains("pre_request = '''"), "{text}");
    assert!(!text.contains("post_response"), "{text}");
    assert_eq!(round_trip(&Scripts::default()).1, "");
}

#[test]
fn missing_and_wrong_tables() {
    assert_eq!(
        read(None, Path::new("x"), "request.scripts").unwrap(),
        Scripts::default()
    );
    let doc: DocumentMut = "scripts = 1".parse().unwrap();
    assert!(read(doc.get(KEY), Path::new("x"), "request.scripts").is_err());
    let doc: DocumentMut = "[scripts]\npre_request = 1".parse().unwrap();
    assert!(read(doc.get(KEY), Path::new("x"), "request.scripts").is_err());
}

#[test]
fn unchanged_scripts_keep_the_table_and_removed_ones_drop_it() {
    let scripts = Scripts {
        pre_request: "log(1)".into(),
        post_response: "log(2)".into(),
    };
    let mut doc: DocumentMut =
        "[scripts]\npre_request = \"log(1)\" # hand-written\npost_response = \"log(2)\"\n"
            .parse()
            .unwrap();
    write(doc.as_table_mut(), &scripts, &scripts);
    assert!(doc.to_string().contains("# hand-written"));
    write(doc.as_table_mut(), &scripts, &Scripts::default());
    assert_eq!(doc.to_string(), "");
}
