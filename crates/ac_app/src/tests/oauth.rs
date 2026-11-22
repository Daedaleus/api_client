use ac_domain::environment::interpolate;
use ac_domain::protocol::HttpMethod;

use super::*;

fn variables(name: &str) -> Option<&'static str> {
    match name {
        "auth" => Some("auth.example.com"),
        "id" => Some("app"),
        "secret" => Some("pw"),
        _ => None,
    }
}

fn fill(text: &str) -> Result<String, UnresolvedVariable> {
    interpolate(text, variables)
}

fn code_flow(redirect_url: &str) -> OAuth2 {
    OAuth2 {
        grant: OAuth2Grant::AuthorizationCode,
        token_url: "https://{{auth}}/token".into(),
        auth_url: "https://{{auth}}/authorize".into(),
        redirect_url: redirect_url.into(),
        client_id: "{{id}}".into(),
        ..OAuth2::default()
    }
}

#[test]
fn client_credentials_resolve_with_variables() {
    let oauth = OAuth2 {
        token_url: "{{auth}}/token".into(),
        client_id: " {{id}} ".into(),
        client_secret: "{{secret}}".into(),
        scope: " read ".into(),
        ..OAuth2::default()
    };
    let client = OAuthClient::resolve(&oauth, &fill).unwrap();
    assert_eq!(client.flow, Flow::ClientCredentials);
    assert_eq!(client.token_url.as_str(), "http://auth.example.com/token");
    assert_eq!(client.client_id, "app");
    assert_eq!(client.client_secret, "pw");
    assert_eq!(client.scope, "read");
    assert!(!client.is_interactive());

    let unknown = OAuth2 {
        client_secret: "{{nope}}".into(),
        ..oauth.clone()
    };
    assert!(matches!(
        OAuthClient::resolve(&unknown, &fill),
        Err(AppError::UnresolvedVariable(_))
    ));
    let empty = OAuth2 {
        token_url: String::new(),
        ..oauth
    };
    assert_eq!(
        OAuthClient::resolve(&empty, &fill).unwrap_err(),
        AppError::EmptyUrl
    );
}

#[test]
fn the_authorization_code_returns_to_a_loopback_address() {
    let client = OAuthClient::resolve(&code_flow(""), &fill).unwrap();
    assert!(client.is_interactive());
    let Flow::AuthorizationCode { auth_url, redirect } = &client.flow else {
        panic!("expected the authorization code");
    };
    assert_eq!(auth_url.as_str(), "https://auth.example.com/authorize");
    assert_eq!(redirect.as_str(), "http://127.0.0.1:0/callback");

    for fine in [
        "http://localhost:8080/callback",
        "http://127.0.0.1:9000/",
        "http://[::1]:9000/cb",
    ] {
        assert!(
            OAuthClient::resolve(&code_flow(fine), &fill).is_ok(),
            "{fine}"
        );
    }
    for wrong in [
        "https://127.0.0.1/cb",
        "http://example.com/cb",
        "http://10.0.0.1/cb",
        "not a url",
    ] {
        assert_eq!(
            OAuthClient::resolve(&code_flow(wrong), &fill).unwrap_err(),
            AppError::RedirectNotLoopback(wrong.into()),
        );
    }
    let text = AppError::RedirectNotLoopback("x".into()).user_text();
    assert_eq!(text.key, keys::OAUTH_REDIRECT);

    let no_auth_url = OAuth2 {
        auth_url: String::new(),
        ..code_flow("")
    };
    assert_eq!(
        OAuthClient::resolve(&no_auth_url, &fill).unwrap_err(),
        AppError::EmptyUrl
    );
}

#[test]
fn debug_output_hides_the_secret() {
    let oauth = OAuth2 {
        client_secret: "{{secret}}".into(),
        ..code_flow("")
    };
    let text = format!("{:?}", OAuthClient::resolve(&oauth, &fill).unwrap());
    assert!(text.contains("app"), "{text}");
    assert!(!text.contains("pw"), "{text}");
}

#[test]
fn errors_explain_themselves() {
    let timeout = TokenError::Login(LoginError::TimedOut).user_text();
    assert_eq!(timeout.key, keys::OAUTH_SIGN_IN_TIMEOUT);
    let browser = TokenError::Login(LoginError::Browser("gone".into())).user_text();
    assert_eq!(browser.key, keys::OAUTH_TOKEN);
    assert_eq!(browser.args["detail"], "cannot open the browser: gone");
    let denied = TokenError::Denied("access_denied".into()).user_text();
    assert_eq!(denied.args["detail"], "sign-in refused: access_denied");
}

#[test]
fn authorize_keeps_an_authorization_header_the_user_set() {
    let mut request = ResolvedRequest {
        transport: ac_domain::transport::EffectiveTransport::default(),
        method: HttpMethod::Get,
        url: Url::parse("https://api.example.com").unwrap(),
        headers: vec![Header::new("Authorization", "Basic x")],
        body: None,
        verify_tls: true,
    };
    authorize(&mut request, "abc");
    assert_eq!(request.headers.len(), 1);

    request.headers.clear();
    authorize(&mut request, "abc");
    assert_eq!(
        request.headers,
        [Header::new(header::AUTHORIZATION, "Bearer abc")]
    );
}
