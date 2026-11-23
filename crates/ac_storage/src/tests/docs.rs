use toml_edit::DocumentMut;

use super::*;

#[test]
fn docs_are_written_as_multi_line_text_and_read_back() {
    let mut doc: DocumentMut = "name = \"Login\"\n".parse().unwrap();
    let text = "# Login\n\nNeeds `{{token}}`.\n";
    write(doc.as_table_mut(), "", text);
    let written = doc.to_string();
    // Multi-line, one line of text per line of the file (diffs well).
    assert!(
        written.contains("\n# Login\n\nNeeds `{{token}}`.\n"),
        "{written}"
    );
    let parsed: DocumentMut = written.parse().unwrap();
    assert_eq!(read(parsed.as_table(), Path::new("x.toml")).unwrap(), text);

    // Unchanged: nothing written; emptied: the key goes.
    let mut same = parsed.clone();
    write(same.as_table_mut(), text, text);
    assert_eq!(same.to_string(), written);
    write(same.as_table_mut(), text, "  ");
    assert_eq!(same.to_string(), "name = \"Login\"\n");
    assert_eq!(read(same.as_table(), Path::new("x.toml")).unwrap(), "");

    let wrong: DocumentMut = "docs = 1\n".parse().unwrap();
    assert!(read(wrong.as_table(), Path::new("x.toml")).is_err());
}
