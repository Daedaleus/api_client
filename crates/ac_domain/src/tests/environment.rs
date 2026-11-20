use super::*;

fn staging() -> Environment {
    let mut environment = Environment::new("Staging");
    environment.variables = vec![
        Variable {
            name: "baseUrl".into(),
            value: "https://staging.example.com".into(),
            enabled: true,
        },
        Variable {
            name: "token".into(),
            value: "old".into(),
            enabled: true,
        },
        Variable {
            name: " token ".into(),
            value: "new".into(),
            enabled: true,
        },
        Variable {
            name: "debug".into(),
            value: "1".into(),
            enabled: false,
        },
    ];
    environment
}

#[test]
fn values_use_enabled_rows_and_the_later_row_wins() {
    let environment = staging();
    let values = environment.values();
    assert_eq!(values.get("baseUrl"), Some(&"https://staging.example.com"));
    assert_eq!(values.get("token"), Some(&"new"));
    assert_eq!(values.get("debug"), None);
}

#[test]
fn placeholders_are_replaced() {
    let environment = staging();
    let values = environment.values();
    let lookup = |name: &str| values.get(name).copied();
    assert_eq!(
        interpolate("{{baseUrl}}/users?token={{ token }}", lookup).unwrap(),
        "https://staging.example.com/users?token=new"
    );
    assert_eq!(
        interpolate("no placeholders", lookup).unwrap(),
        "no placeholders"
    );
    assert_eq!(interpolate("", lookup).unwrap(), "");
}

#[test]
fn unclosed_placeholders_stay_and_values_are_not_resolved_again() {
    let lookup = |name: &str| match name {
        "a" => Some("{{b}}"),
        _ => None,
    };
    assert_eq!(interpolate("x {{a}} {{ y", lookup).unwrap(), "x {{b}} {{ y");
}

#[test]
fn unknown_variables_are_reported() {
    let error = interpolate("{{baseUrl}}/{{missing}}", |name| {
        (name == "baseUrl").then_some("u")
    })
    .unwrap_err();
    assert_eq!(error, UnresolvedVariable("missing".into()));
    assert!(error.to_string().contains("not defined"));
}
