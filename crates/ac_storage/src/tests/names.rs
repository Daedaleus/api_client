use super::*;

#[test]
fn slugs_are_readable_file_names() {
    assert_eq!(slug("Get product", "request"), "get-product");
    assert_eq!(slug("Größe ändern!", "request"), "groesse-aendern");
    assert_eq!(slug("  --A  B--  ", "request"), "a-b");
    assert_eq!(slug("????", "request"), "request");
    assert_eq!(slug("", "folder"), "folder");
    assert_eq!(slug("CON", "request"), "request-con");
    assert_eq!(slug(&"x".repeat(100), "request").len(), 60);
    assert_eq!(
        slug(&format!("{} y", "x".repeat(59)), "request"),
        "x".repeat(59)
    );
}

#[test]
fn unique_names_count_up() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    assert_eq!(unique_file(dir, "a"), dir.join("a.toml"));
    std::fs::write(dir.join("a.toml"), "").unwrap();
    assert_eq!(unique_file(dir, "a"), dir.join("a-2.toml"));
    std::fs::create_dir(dir.join("b")).unwrap();
    assert_eq!(unique_dir(dir, "b"), dir.join("b-2"));
}

#[test]
fn file_names_and_stems() {
    let path = Path::new("x").join("get-product.toml");
    assert_eq!(file_name(&path), "get-product.toml");
    assert_eq!(file_stem(&path), "get-product");
    assert_eq!(file_name(Path::new("")), "");
}
