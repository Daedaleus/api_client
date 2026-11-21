use super::*;

fn url() -> Url {
    Url::parse("https://auth.example.com/authorize?a=1&b=2").unwrap()
}

fn parts(command: &Command) -> (String, Vec<String>) {
    (
        command.get_program().to_string_lossy().into_owned(),
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
    )
}

#[test]
fn the_browser_variable_wins_and_gets_the_url_as_last_argument() {
    let node = command(&url(), Some(r#"node "my script.mjs" --flag"#)).unwrap();
    assert_eq!(
        parts(&node),
        (
            "node".to_owned(),
            vec![
                "my script.mjs".to_owned(),
                "--flag".to_owned(),
                url().to_string()
            ]
        )
    );
    assert!(command(&url(), Some("\"unclosed")).is_err());
}

#[test]
fn otherwise_the_system_opens_the_url() {
    for browser in [None, Some("  ")] {
        let (program, arguments) = parts(&command(&url(), browser).unwrap());
        assert_ne!(program, "");
        assert_eq!(arguments.last(), Some(&url().to_string()));
    }
}

#[test]
fn a_missing_program_is_reported() {
    let result = command(&url(), Some("this-program-does-not-exist-4711"))
        .unwrap()
        .spawn();
    assert!(result.is_err());
}
