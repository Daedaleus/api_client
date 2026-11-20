use ac_domain::environment::Variable;

use super::*;

#[test]
fn drafts_round_trip_to_environments() {
    let mut environment = Environment::new("Staging");
    environment.variables.push(Variable {
        name: "baseUrl".into(),
        value: "https://staging.example.com".into(),
        enabled: false,
        secret: false,
    });
    let draft = EnvironmentDraft::from_environment(&environment, true);
    assert!(draft.read_only);
    assert_eq!(draft.to_environment().unwrap(), environment);
}

#[test]
fn drafts_match_the_ui_contract() {
    let json = serde_json::json!({
        "id": "not-an-id",
        "name": "Staging",
        "variables": [{ "name": "a", "value": "b", "enabled": true }]
    });
    let draft: EnvironmentDraft = serde_json::from_value(json).unwrap();
    assert!(!draft.read_only);
    assert_eq!(draft.to_environment().unwrap_err(), AppError::UnknownItem);

    let context: crate::view::SendContext = serde_json::from_value(serde_json::json!({
        "collectionId": "c",
        "environmentId": "e"
    }))
    .unwrap();
    assert_eq!(context.environment_id.as_deref(), Some("e"));
    assert_eq!(context.request_id, None);
}

#[test]
fn secret_values_never_reach_the_ui() {
    let mut environment = Environment::new("Staging");
    environment.variables.push(Variable {
        name: "token".into(),
        value: "s3cr3t".into(),
        enabled: true,
        secret: true,
    });
    let draft = EnvironmentDraft::from_environment(&environment, false);
    assert!(draft.variables[0].secret);
    assert_eq!(draft.variables[0].value, "");
    let json = serde_json::to_string(&draft).unwrap();
    assert!(!json.contains("s3cr3t"), "{json}");
}
