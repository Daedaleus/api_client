use super::*;

#[test]
fn types_are_removed_and_enums_lowered() {
    let js = strip("enum S { Ok = 200 }\nconst x: number = S.Ok as number;\nlet y = x!;").unwrap();
    assert!(!js.contains(": number"), "{js}");
    assert!(js.contains("S[S[\"Ok\"] = 200]"), "{js}");
}

#[test]
fn syntax_errors_name_their_line() {
    assert_eq!(
        strip("log(1);\nconst x = ;").unwrap_err(),
        ScriptError::Syntax {
            line: 2,
            message: "Unexpected token".into()
        }
    );
    assert_eq!(
        syntax_error("ab", None, "x"),
        ScriptError::Syntax {
            line: 1,
            message: "x".into()
        }
    );
    assert_eq!(
        syntax_error("a\nb", Some(99), "x"),
        ScriptError::Syntax {
            line: 2,
            message: "x".into()
        }
    );
}
