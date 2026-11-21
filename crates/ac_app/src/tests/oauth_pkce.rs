use std::collections::HashMap;

use super::*;
use crate::oauth::Flow;

fn client(scope: &str) -> OAuthClient {
    OAuthClient {
        flow: Flow::ClientCredentials,
        token_url: Url::parse("https://auth.example.com/token").unwrap(),
        client_id: "app".into(),
        client_secret: String::new(),
        scope: scope.into(),
    }
}

#[test]
fn the_challenge_is_the_sha256_of_the_verifier() {
    // Computed independently (Python: hashlib + urlsafe_b64encode).
    assert_eq!(
        challenge("dBjftJeZ4CVP-mJ92K9iMLsCBDOIsJ5oFtSu_9I3s3s"),
        "FtuZuA26RrSHUzpnCDtTVj3A_VOrTYJt7vlcOVZ2vPw"
    );
}

#[test]
fn the_sign_in_url_carries_client_redirect_state_and_challenge() {
    let auth_url = Url::parse("https://auth.example.com/authorize?audience=shop").unwrap();
    let redirect = Url::parse("http://127.0.0.1:4711/callback").unwrap();
    let sign_in = SignIn::new(&client("read write"), &auth_url, &redirect);
    assert_eq!(sign_in.verifier.len(), 43);
    assert_eq!(sign_in.state.len(), 22);
    let query: HashMap<_, _> = sign_in.url.query_pairs().into_owned().collect();
    assert_eq!(query["audience"], "shop");
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], "app");
    assert_eq!(query["redirect_uri"], "http://127.0.0.1:4711/callback");
    assert_eq!(query["scope"], "read write");
    assert_eq!(query["state"], sign_in.state);
    assert_eq!(query["code_challenge"], challenge(&sign_in.verifier));
    assert_eq!(query["code_challenge_method"], "S256");

    let again = SignIn::new(&client(""), &auth_url, &redirect);
    assert_ne!(again.verifier, sign_in.verifier);
    assert_ne!(again.state, sign_in.state);
    assert!(!again.url.query_pairs().any(|(key, _)| key == "scope"));
}
