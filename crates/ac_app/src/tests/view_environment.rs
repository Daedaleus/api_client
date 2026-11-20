use super::*;

#[test]
fn drafts_round_trip_to_environments() {
    let mut environment = Environment::new("Staging");
    environment.variables.push(Variable {
        name: "baseUrl".into(),
        value: "https://staging.example.com".into(),
        enabled: false,
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

    let reference: EnvironmentRef = serde_json::from_value(serde_json::json!({
        "collectionId": "c",
        "environmentId": "e"
    }))
    .unwrap();
    assert_eq!(reference.environment_id, "e");
}
