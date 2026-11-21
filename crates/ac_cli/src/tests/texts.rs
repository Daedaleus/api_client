use super::*;

#[test]
fn every_cli_text_exists_in_every_language_with_its_placeholders() {
    for language in LANGUAGES {
        let texts = Texts::new(language);
        for (key, placeholders) in keys::ALL {
            let entry = texts
                .messages
                .get(key)
                .unwrap_or_else(|| panic!("{key} is missing"));
            let text = entry
                .get(language)
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{key} has no {language} text"));
            for placeholder in placeholders {
                assert!(
                    text.contains(&format!("{{{placeholder}}}")),
                    "{key} ({language})"
                );
            }
        }
    }
}

#[test]
fn the_language_comes_from_the_locale_and_falls_back_to_english() {
    assert_eq!(Texts::new("de_DE.UTF-8").language, "de");
    assert_eq!(Texts::new("en_US").language, "en");
    assert_eq!(Texts::new("fr_FR").language, "en");
    assert_eq!(Texts::new("").language, "en");
    let _ = Texts::from_environment();
}

#[test]
fn texts_fill_their_placeholders() {
    let texts = Texts::new("en");
    assert_eq!(
        texts.get(
            keys::SUMMARY,
            &[("total", "3"), ("passed", "2"), ("failed", "1")]
        ),
        "3 requests: 2 passed, 1 failed"
    );
    assert_eq!(texts.get("no.such.key", &[]), "no.such.key");
    let error = UserText::new("error.unknown_environment").with("name", "Prod");
    assert_eq!(texts.error(&error), "Error: No environment named “Prod”.");
    assert_eq!(
        Texts::new("de").error(&error),
        "Fehler: Keine Umgebung namens „Prod“."
    );
}
