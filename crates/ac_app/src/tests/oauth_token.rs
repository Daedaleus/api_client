use super::*;
use crate::oauth::Flow;
use crate::ports::TransportError;
use crate::testing::response;

fn client(secret: &str) -> OAuthClient {
    OAuthClient {
        flow: Flow::ClientCredentials,
        token_url: Url::parse("https://auth.example.com/token").unwrap(),
        client_id: "shop app".into(),
        client_secret: secret.into(),
        scope: "read write".into(),
    }
}

fn header<'a>(request: &'a ResolvedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

fn body(request: &ResolvedRequest) -> &str {
    std::str::from_utf8(request.body_bytes().unwrap()).unwrap()
}

#[test]
fn client_credentials_with_basic_client_authentication() {
    let request = token_request(&client("s3cr:t"), &TokenGrant::ClientCredentials, false);
    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.url.as_str(), "https://auth.example.com/token");
    assert!(!request.verify_tls);
    // "shop+app:s3cr%3At", form-encoded before Base64.
    assert_eq!(
        header(&request, header::AUTHORIZATION),
        Some("Basic c2hvcCthcHA6czNjciUzQXQ=")
    );
    assert_eq!(
        header(&request, header::CONTENT_TYPE),
        Some(media_type::FORM)
    );
    assert_eq!(
        body(&request),
        "grant_type=client_credentials&scope=read+write"
    );
}

#[test]
fn a_public_client_names_its_id_in_the_form() {
    let redirect = Url::parse("http://127.0.0.1:4711/callback").unwrap();
    let grant = TokenGrant::AuthorizationCode {
        code: "c0de",
        redirect_uri: &redirect,
        verifier: "v",
    };
    let request = token_request(&client(""), &grant, true);
    assert_eq!(header(&request, header::AUTHORIZATION), None);
    assert_eq!(
        body(&request),
        "grant_type=authorization_code&code=c0de\
         &redirect_uri=http%3A%2F%2F127.0.0.1%3A4711%2Fcallback&code_verifier=v\
         &client_id=shop+app"
    );

    let refresh = token_request(
        &client(""),
        &TokenGrant::Refresh { refresh_token: "r" },
        true,
    );
    assert_eq!(
        body(&refresh),
        "grant_type=refresh_token&refresh_token=r&scope=read+write&client_id=shop+app"
    );

    let no_scope = OAuthClient {
        scope: String::new(),
        ..client("x")
    };
    assert_eq!(
        body(&token_request(
            &no_scope,
            &TokenGrant::ClientCredentials,
            true
        )),
        "grant_type=client_credentials"
    );
}

#[test]
fn reads_token_lifetime_and_refresh_token() {
    let answer = |body: &str| parse(&response(200, media_type::JSON, body));
    assert_eq!(
        answer(r#"{"access_token":"a","expires_in":3600,"refresh_token":"r"}"#).unwrap(),
        TokenResponse {
            access_token: "a".into(),
            expires_in: Some(Duration::from_secs(3600)),
            refresh_token: Some("r".into()),
        }
    );
    assert_eq!(
        answer(r#"{"access_token":"a","expires_in":"60","refresh_token":""}"#).unwrap(),
        TokenResponse {
            access_token: "a".into(),
            expires_in: Some(Duration::from_secs(60)),
            refresh_token: None,
        }
    );
    assert_eq!(answer(r#"{"token":"x"}"#), Err(TokenError::NoToken));
    assert_eq!(answer("not json"), Err(TokenError::NoToken));
}

#[test]
fn reports_why_the_server_refused() {
    let refused = |status, body: &str| parse(&response(status, media_type::JSON, body));
    let reason = |status, reason: &str| {
        Err(TokenError::Refused {
            status,
            reason: reason.into(),
        })
    };
    assert_eq!(
        refused(
            401,
            r#"{"error":"invalid_client","error_description":"unknown client"}"#
        ),
        reason(401, "invalid_client (unknown client)")
    );
    assert_eq!(
        refused(400, r#"{"error":"invalid_grant"}"#),
        reason(400, "invalid_grant")
    );
    assert_eq!(
        refused(502, "<h1>Bad gateway</h1>"),
        reason(502, "<h1>Bad gateway</h1>")
    );
    let _ = TransportError::Timeout;
}
