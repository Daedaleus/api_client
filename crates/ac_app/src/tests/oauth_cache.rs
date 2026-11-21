use std::collections::HashMap;

use ac_domain::exchange::{ResolvedRequest, Response};
use ac_domain::protocol::media_type;

use super::*;
use crate::ports::{LoginError, TransportError};
use crate::testing::{FAKE_PORT, FakeBrowserLogin, FakeHttpSender, LoginAnswer, response};

fn credentials() -> OAuthClient {
    OAuthClient {
        flow: Flow::ClientCredentials,
        token_url: Url::parse("https://auth.example.com/token").unwrap(),
        client_id: "app".into(),
        client_secret: "pw".into(),
        scope: "read".into(),
    }
}

fn code_flow() -> OAuthClient {
    OAuthClient {
        flow: Flow::AuthorizationCode {
            auth_url: Url::parse("https://auth.example.com/authorize").unwrap(),
            redirect: Url::parse("http://127.0.0.1:0/callback").unwrap(),
        },
        client_secret: String::new(),
        ..credentials()
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "queued as a reply of the fake sender"
)]
fn token(body: &str) -> Result<Response, TransportError> {
    Ok(response(200, media_type::JSON, body))
}

fn form(request: &ResolvedRequest) -> HashMap<String, String> {
    url::form_urlencoded::parse(request.body_bytes().unwrap())
        .into_owned()
        .collect()
}

async fn get(
    cache: &TokenCache,
    http: &FakeHttpSender,
    login: &FakeBrowserLogin,
    client: &OAuthClient,
) -> Result<String, TokenError> {
    cache.token(http, login, client, true).await
}

#[tokio::test]
async fn keeps_a_token_until_shortly_before_it_expires() {
    let http = FakeHttpSender::new()
        .reply(token(r#"{"access_token":"abc","expires_in":3600}"#))
        .reply(token(r#"{"access_token":"new"}"#));
    let (cache, login) = (TokenCache::default(), FakeBrowserLogin::new());
    for _ in 0..2 {
        assert_eq!(
            get(&cache, &http, &login, &credentials()).await.unwrap(),
            "abc"
        );
    }
    assert_eq!(http.sent().len(), 1);
    assert_eq!(form(&http.sent()[0])["grant_type"], "client_credentials");

    // Other settings, other token.
    let other = OAuthClient {
        scope: "admin".into(),
        ..credentials()
    };
    assert_eq!(get(&cache, &http, &login, &other).await.unwrap(), "new");
    assert_eq!(http.sent().len(), 2);
}

#[tokio::test]
async fn client_credentials_without_a_usable_lifetime_are_fetched_anew() {
    let http = FakeHttpSender::new()
        .reply(token(r#"{"access_token":"a","expires_in":"20"}"#))
        .reply(token(r#"{"access_token":"b"}"#))
        .reply(token(r#"{"access_token":"c","expires_in":"60"}"#));
    let (cache, login) = (TokenCache::default(), FakeBrowserLogin::new());
    for expected in ["a", "b", "c", "c"] {
        assert_eq!(
            get(&cache, &http, &login, &credentials()).await.unwrap(),
            expected
        );
    }
    assert_eq!(http.sent().len(), 3);
}

#[tokio::test]
async fn signs_in_in_the_browser_and_exchanges_the_code() {
    let http = FakeHttpSender::new().reply(token(r#"{"access_token":"user-token"}"#));
    let login = FakeBrowserLogin::new().answer(LoginAnswer::Approve("c0de".into()));
    let cache = TokenCache::default();
    for _ in 0..2 {
        assert_eq!(
            get(&cache, &http, &login, &code_flow()).await.unwrap(),
            "user-token"
        );
    }
    // Without a lifetime a signed-in token is kept: no second sign-in.
    assert_eq!(login.opened().len(), 1);
    let sent = http.sent();
    assert_eq!(sent.len(), 1);
    let exchange = form(&sent[0]);
    let opened: HashMap<_, _> = login.opened()[0].query_pairs().into_owned().collect();
    assert_eq!(exchange["grant_type"], "authorization_code");
    assert_eq!(exchange["code"], "c0de");
    assert_eq!(exchange["client_id"], "app");
    let redirect = format!("http://127.0.0.1:{FAKE_PORT}/callback");
    assert_eq!(exchange["redirect_uri"], redirect);
    assert_eq!(opened["redirect_uri"], redirect);
    assert_eq!(
        opened["code_challenge"],
        super::super::pkce::challenge(&exchange["code_verifier"])
    );
}

#[tokio::test]
async fn refreshes_an_expired_token_and_signs_in_again_if_that_fails() {
    let http = FakeHttpSender::new()
        .reply(token(
            r#"{"access_token":"t1","expires_in":10,"refresh_token":"r1"}"#,
        ))
        // Refreshed; the refresh token stays as it was.
        .reply(token(r#"{"access_token":"t2","expires_in":10}"#))
        // The refresh token was revoked …
        .reply(Ok(response(
            400,
            media_type::JSON,
            r#"{"error":"invalid_grant"}"#,
        )))
        // … so the user signs in again.
        .reply(token(r#"{"access_token":"t3","expires_in":3600}"#));
    let login = FakeBrowserLogin::new()
        .answer(LoginAnswer::Approve("one".into()))
        .answer(LoginAnswer::Approve("two".into()));
    let cache = TokenCache::default();
    for expected in ["t1", "t2", "t3", "t3"] {
        assert_eq!(
            get(&cache, &http, &login, &code_flow()).await.unwrap(),
            expected
        );
    }
    let grants: Vec<_> = http
        .sent()
        .iter()
        .map(|request| form(request)["grant_type"].clone())
        .collect();
    assert_eq!(
        grants,
        [
            "authorization_code",
            "refresh_token",
            "refresh_token",
            "authorization_code"
        ]
    );
    assert_eq!(form(&http.sent()[2])["refresh_token"], "r1");
    assert_eq!(login.opened().len(), 2);
}

#[tokio::test]
async fn a_failed_sign_in_says_why() {
    let http = FakeHttpSender::new();
    let login = FakeBrowserLogin::new()
        .answer(LoginAnswer::Deny("access_denied".into()))
        .answer(LoginAnswer::WrongState("c0de".into()))
        .answer(LoginAnswer::NoCode)
        .answer(LoginAnswer::Fail(LoginError::Browser("none".into())));
    let cache = TokenCache::default();
    let mut errors = Vec::new();
    for _ in 0..5 {
        errors.push(get(&cache, &http, &login, &code_flow()).await.unwrap_err());
    }
    assert_eq!(
        errors,
        [
            TokenError::Denied("access_denied".into()),
            TokenError::StateMismatch,
            TokenError::NoCode,
            TokenError::Login(LoginError::Browser("none".into())),
            // Nobody answered.
            TokenError::Login(LoginError::TimedOut),
        ]
    );
    assert_eq!(http.sent().len(), 0);

    let busy = FakeBrowserLogin::new().fail_listen(LoginError::Listen {
        address: "127.0.0.1:8080".into(),
        detail: "in use".into(),
    });
    assert!(matches!(
        get(&cache, &http, &busy, &code_flow()).await,
        Err(TokenError::Login(LoginError::Listen { .. }))
    ));
}

#[tokio::test]
async fn reports_why_no_token_came() {
    let http = FakeHttpSender::new()
        .reply(Ok(response(
            401,
            media_type::JSON,
            r#"{"error":"invalid_client"}"#,
        )))
        .reply(Err(TransportError::Timeout));
    let (cache, login) = (TokenCache::default(), FakeBrowserLogin::new());
    assert_eq!(
        get(&cache, &http, &login, &credentials()).await,
        Err(TokenError::Refused {
            status: 401,
            reason: "invalid_client".into()
        })
    );
    assert_eq!(
        get(&cache, &http, &login, &credentials()).await,
        Err(TokenError::Transport(TransportError::Timeout))
    );
}
