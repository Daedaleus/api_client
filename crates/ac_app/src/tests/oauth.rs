use ac_domain::environment::interpolate;

use super::*;
use crate::testing::{FakeHttpSender, response};

fn credentials() -> ClientCredentials {
    ClientCredentials {
        token_url: Url::parse("https://auth.example.com/token").unwrap(),
        client_id: "shop app".into(),
        client_secret: "s3cr:t".into(),
        scope: "read write".into(),
    }
}

fn token_reply(body: &str) -> Response {
    response(200, media_type::JSON, body)
}

fn header<'a>(request: &'a ResolvedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

#[test]
fn resolves_variables_and_checks_the_token_url() {
    let oauth = OAuth2 {
        token_url: "{{auth}}/token".into(),
        client_id: " {{id}} ".into(),
        client_secret: "{{secret}}".into(),
        scope: String::new(),
    };
    let fill = |text: &str| {
        interpolate(text, |name| match name {
            "auth" => Some("auth.example.com"),
            "id" => Some("app"),
            "secret" => Some("pw"),
            _ => None,
        })
    };
    let resolved = ClientCredentials::resolve(&oauth, &fill).unwrap();
    assert_eq!(resolved.token_url.as_str(), "http://auth.example.com/token");
    assert_eq!(resolved.client_id, "app");
    assert_eq!(resolved.client_secret, "pw");

    let unknown = OAuth2 {
        client_secret: "{{nope}}".into(),
        ..oauth.clone()
    };
    assert!(matches!(
        ClientCredentials::resolve(&unknown, &fill),
        Err(AppError::UnresolvedVariable(_))
    ));
    let empty = OAuth2 {
        token_url: String::new(),
        ..oauth
    };
    assert_eq!(
        ClientCredentials::resolve(&empty, &fill).unwrap_err(),
        AppError::EmptyUrl
    );
}

#[test]
fn debug_output_hides_the_secret() {
    let text = format!("{:?}", credentials());
    assert!(text.contains("shop app"));
    assert!(!text.contains("s3cr"));
}

#[test]
fn token_request_is_a_form_post_with_basic_client_authentication() {
    let request = credentials().token_request(false);
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
        request.body_bytes(),
        Some(&b"grant_type=client_credentials&scope=read+write"[..])
    );

    let no_scope = ClientCredentials {
        scope: String::new(),
        ..credentials()
    };
    assert_eq!(
        no_scope.token_request(true).body_bytes(),
        Some(&b"grant_type=client_credentials"[..])
    );
}

#[tokio::test]
async fn keeps_a_token_until_shortly_before_it_expires() {
    let http = FakeHttpSender::new()
        .reply(Ok(token_reply(
            r#"{"access_token":"abc","expires_in":3600}"#,
        )))
        .reply(Ok(token_reply(r#"{"access_token":"new"}"#)));
    let cache = TokenCache::default();
    assert_eq!(
        cache.token(&http, &credentials(), true).await.unwrap(),
        "abc"
    );
    assert_eq!(
        cache.token(&http, &credentials(), true).await.unwrap(),
        "abc"
    );
    assert_eq!(http.sent().len(), 1);

    // Other credentials, other token.
    let other = ClientCredentials {
        scope: "admin".into(),
        ..credentials()
    };
    assert_eq!(cache.token(&http, &other, true).await.unwrap(), "new");
    assert_eq!(http.sent().len(), 2);
}

#[tokio::test]
async fn fetches_anew_without_a_usable_lifetime() {
    let http = FakeHttpSender::new()
        // Expires within the margin.
        .reply(Ok(token_reply(r#"{"access_token":"a","expires_in":"20"}"#)))
        // No lifetime at all.
        .reply(Ok(token_reply(r#"{"access_token":"b"}"#)))
        .reply(Ok(token_reply(r#"{"access_token":"c","expires_in":"60"}"#)));
    let cache = TokenCache::default();
    for expected in ["a", "b", "c", "c"] {
        assert_eq!(
            cache.token(&http, &credentials(), true).await.unwrap(),
            expected
        );
    }
    assert_eq!(http.sent().len(), 3);
}

#[tokio::test]
async fn reports_why_no_token_came() {
    let http = FakeHttpSender::new()
        .reply(Ok(response(
            401,
            media_type::JSON,
            r#"{"error":"invalid_client","error_description":"unknown client"}"#,
        )))
        .reply(Ok(response(
            400,
            media_type::JSON,
            r#"{"error":"invalid_scope"}"#,
        )))
        .reply(Ok(response(502, "text/html", "<h1>Bad gateway</h1>")))
        .reply(Ok(token_reply(r#"{"token":"x"}"#)))
        .reply(Err(TransportError::Timeout));
    let cache = TokenCache::default();
    let mut errors = Vec::new();
    for _ in 0..5 {
        errors.push(cache.token(&http, &credentials(), true).await.unwrap_err());
    }
    let refused = |status, reason: &str| TokenError::Refused {
        status,
        reason: reason.into(),
    };
    assert_eq!(
        errors,
        [
            refused(401, "invalid_client (unknown client)"),
            refused(400, "invalid_scope"),
            refused(502, "<h1>Bad gateway</h1>"),
            TokenError::NoToken,
            TokenError::Transport(TransportError::Timeout),
        ]
    );
    let text = errors[0].user_text();
    assert_eq!(text.key, keys::OAUTH_TOKEN);
    assert_eq!(
        text.args["detail"],
        "status 401: invalid_client (unknown client)"
    );
}

#[test]
fn authorize_keeps_an_authorization_header_the_user_set() {
    let mut request = credentials().token_request(true);
    authorize(&mut request, "abc");
    assert!(
        header(&request, header::AUTHORIZATION)
            .unwrap()
            .starts_with("Basic")
    );

    request.headers.clear();
    authorize(&mut request, "abc");
    assert_eq!(header(&request, header::AUTHORIZATION), Some("Bearer abc"));
}
