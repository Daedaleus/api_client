use std::path::PathBuf;

use toml_edit::Table;

use super::*;

#[test]
fn multi_line_text_becomes_a_literal_block() {
    let value = text_value("a\nb\n");
    assert_eq!(value.to_string().trim(), "'''\na\nb\n'''");
    assert_eq!(value.as_str(), Some("a\nb\n"));
}

#[test]
fn other_text_stays_a_normal_string() {
    assert_eq!(text_value("one line").to_string().trim(), "\"one line\"");
    let tricky = text_value("has ''' and\nnewline");
    assert_eq!(tricky.as_str(), Some("has ''' and\nnewline"));
    assert!(!tricky.to_string().contains("'''\n"));
}

#[test]
fn set_keeps_the_comment_of_a_changed_value_and_leaves_equal_ones() {
    let mut doc: DocumentMut = "a = \"x\" # note\nb = 1 # keep\n".parse().unwrap();
    let table = doc.as_table_mut();
    set(table, "a", "y");
    set(table, "b", 1);
    set(table, "c", true);
    assert_eq!(
        doc.to_string(),
        "a = \"y\" # note\nb = 1 # keep\nc = true\n"
    );

    let mut doc: DocumentMut = "t = 1\n".parse().unwrap();
    let mut sub = Table::new();
    sub.insert("x", toml_edit::value(1));
    doc.insert("t", Item::Table(sub));
    set(doc.as_table_mut(), "t", "now a string");
    assert_eq!(doc.get("t").and_then(Item::as_str), Some("now a string"));
}

#[test]
fn order_must_be_an_array_of_strings() {
    let path = PathBuf::from("folder.toml");
    let doc: DocumentMut = "order = \"a\"".parse().unwrap();
    assert!(order(doc.as_table(), &path).is_err());
    let doc: DocumentMut = "order = [\"a\", \"b\"]".parse().unwrap();
    assert_eq!(order(doc.as_table(), &path).unwrap(), ["a", "b"]);
}
