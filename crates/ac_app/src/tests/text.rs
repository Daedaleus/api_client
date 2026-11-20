use std::collections::BTreeSet;

use serde_json::Value;

use super::*;

const LOCALES: &str = include_str!("../../../../locales/app.json");
const LANGUAGES: [&str; 2] = ["de", "en"];

fn placeholders(text: &str) -> BTreeSet<String> {
    text.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_owned()))
        .collect()
}

#[test]
fn every_key_used_here_exists_with_its_placeholders_in_all_languages() {
    let locales: Value = serde_json::from_str(LOCALES).expect("locales/app.json is valid JSON");
    for (key, args) in keys::ALL {
        let entry = &locales[key];
        let expected: BTreeSet<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        for language in LANGUAGES {
            let text = entry[language]
                .as_str()
                .unwrap_or_else(|| panic!("{key} has no {language} text"));
            assert_eq!(placeholders(text), expected, "{key} ({language})");
        }
    }
}

#[test]
fn user_text_collects_arguments() {
    let text = UserText::new(keys::CONNECT).with("detail", "refused");
    assert_eq!(text.key, "error.connect");
    assert_eq!(text.args.get("detail").map(String::as_str), Some("refused"));
    assert_eq!(
        serde_json::to_value(&text).unwrap(),
        serde_json::json!({ "key": "error.connect", "args": { "detail": "refused" } })
    );
}
