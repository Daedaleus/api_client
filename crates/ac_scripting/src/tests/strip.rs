use super::*;

#[test]
fn types_are_removed_and_enums_lowered() {
    let js = strip("enum S { Ok = 200 }\nconst x: number = S.Ok as number;\nlet y = x!;")
        .unwrap()
        .code;
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

#[test]
fn generated_lines_lead_back_to_the_script() {
    // Type-only lines vanish and the code is re-formatted.
    let source = "type A = { x: number };\n\nconst a: A = { x: 1 };\nlog(a.x);\nnull.y;";
    let stripped = strip(source).unwrap();
    let lines: Vec<&str> = stripped.code.lines().collect();
    let generated = lines
        .iter()
        .position(|line| line.contains("null.y"))
        .unwrap();
    let generated = u32::try_from(generated).unwrap();
    assert_eq!(
        stripped.source_line(generated, 0),
        Some(5),
        "{}",
        stripped.code
    );
    assert_eq!(stripped.source_line(generated, 99), Some(5));
    assert_eq!(stripped.source_line(999, 0), None);
}
