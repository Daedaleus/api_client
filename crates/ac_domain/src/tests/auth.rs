use super::*;

fn bearer(token: &str) -> Auth {
    Auth::Bearer {
        token: token.into(),
    }
}

#[test]
fn the_first_auth_that_does_not_inherit_applies() {
    let request = Auth::Inherit;
    let folder = bearer("folder");
    let collection = bearer("collection");
    assert_eq!(effective([&request, &folder, &collection]), &folder);
    assert_eq!(effective([&Auth::None, &folder]), &Auth::None);
    assert_eq!(effective([&Auth::Inherit, &Auth::Inherit]), &Auth::None);
    assert_eq!(effective([]), &Auth::None);
    assert_eq!(Auth::default(), Auth::Inherit);
}

#[test]
fn credentials_stay_out_of_debug_output() {
    let all = [
        Auth::Inherit,
        Auth::None,
        Auth::Basic {
            username: "alice".into(),
            password: "hunter2".into(),
        },
        bearer("s3cr3t"),
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "k3y".into(),
            placement: KeyPlacement::Query,
        },
        Auth::OAuth2(OAuth2 {
            token_url: "https://auth.example.com/token".into(),
            client_id: "app".into(),
            client_secret: "cl1ent".into(),
            scope: String::new(),
        }),
    ];
    let debug = format!("{all:?}");
    assert!(debug.contains("auth.example.com"), "{debug}");
    for secret in ["hunter2", "s3cr3t", "k3y", "cl1ent"] {
        assert!(!debug.contains(secret), "{debug}");
    }
    assert!(debug.contains("alice") && debug.contains("X-API-Key") && debug.contains("Query"));
}
