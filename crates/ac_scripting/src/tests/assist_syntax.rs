use super::*;

#[test]
fn syntax_errors_have_their_position() {
    let found = diagnostics("log(1);\nconst = ;");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Error);
    assert!(
        found[0].from >= 8 && found[0].to > found[0].from,
        "{found:?}"
    );
    assert_ne!(found[0].message, "");
}

#[test]
fn correct_scripts_have_none() {
    assert_eq!(
        diagnostics("const a: string = 'x';\nawait Promise.resolve(a);"),
        []
    );
}

#[test]
fn offsets_count_as_javascript_does() {
    // "ä" is two bytes, one UTF-16 unit; "😀" four bytes, two units.
    let ascii = diagnostics("const a = 'x';\nconst = ;");
    let wide = diagnostics("const ä = '😀';\nconst = ;");
    assert_eq!(wide[0].from, ascii[0].from + 1, "{ascii:?} {wide:?}");
    assert_eq!(utf16_offset("ä😀x", 6), 3);
    // Inside a character: from its start; past the end: the end.
    assert_eq!(utf16_offset("ä", 1), 0);
    assert_eq!(utf16_offset("ab", 9), 2);
}
