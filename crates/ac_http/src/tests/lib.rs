use ac_domain::protocol::HttpMethod;
use url::Url;
use wiremock::matchers::{body_string, body_string_contains, header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn request(method: HttpMethod, url: &str, headers: Vec<Header>) -> ResolvedRequest {
    ResolvedRequest {
        transport: ac_domain::transport::EffectiveTransport::default(),
        verify_tls: true,
        method,
        url: Url::parse(url).unwrap(),
        headers,
        body: None,
    }
}

#[tokio::test]
async fn sends_the_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("content-type", "application/json"))
        .and(body_string("{\"name\":\"Teapot\"}"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    let mut post = request(
        HttpMethod::Post,
        &server.uri(),
        vec![Header::new("Content-Type", "application/json")],
    );
    post.body = Some(RequestBody::Bytes(b"{\"name\":\"Teapot\"}".to_vec()));
    let response = sender().send(post, None).await.unwrap();
    assert_eq!(response.status.as_u16(), 201);
}

fn sender() -> ReqwestSender {
    ReqwestSender::new(DEFAULT_TIMEOUT).unwrap()
}

#[tokio::test]
async fn sends_method_headers_and_reads_the_response() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/items/1"))
        .and(header("x-test", "yes"))
        .and(header("user-agent", USER_AGENT))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("x-id", "42")
                .set_body_string("created"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/items/1", server.uri());
    let response = sender()
        .send(
            request(HttpMethod::Put, &url, vec![Header::new("X-Test", "yes")]),
            None,
        )
        .await
        .unwrap();

    assert_eq!(response.status.as_u16(), 201);
    assert_eq!(response.header("X-Id"), Some("42"));
    assert_eq!(response.body, b"created");
}

#[tokio::test]
async fn error_statuses_are_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let response = sender()
        .send(request(HttpMethod::Get, &server.uri(), vec![]), None)
        .await
        .unwrap();
    assert_eq!(response.status.as_u16(), 503);
    assert_eq!(response.body, b"");
}

#[tokio::test]
async fn custom_methods_are_sent() {
    let server = MockServer::start().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;

    let method = HttpMethod::Custom("PROPFIND".into());
    let response = sender()
        .send(request(method, &server.uri(), vec![]), None)
        .await
        .unwrap();
    assert_eq!(response.status.as_u16(), 207);
}

#[tokio::test]
async fn slow_servers_time_out() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    // The request's own time limit counts.
    let mut slow = request(HttpMethod::Get, &server.uri(), vec![]);
    slow.transport.timeout = Duration::from_millis(100);
    let error = sender().send(slow, None).await.unwrap_err();
    assert_eq!(error, TransportError::Timeout);
}

#[tokio::test]
async fn redirects_are_followed_unless_the_request_says_stop() {
    let server = MockServer::start().await;
    Mock::given(path("/old"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/new"))
        .mount(&server)
        .await;
    Mock::given(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_string("moved"))
        .mount(&server)
        .await;

    let url = format!("{}/old", server.uri());
    let followed = sender()
        .send(request(HttpMethod::Get, &url, vec![]), None)
        .await
        .unwrap();
    assert_eq!(followed.status.as_u16(), 200);
    assert_eq!(followed.body, b"moved");

    let mut stop = request(HttpMethod::Get, &url, vec![]);
    stop.transport.follow_redirects = false;
    let redirect = sender().send(stop, None).await.unwrap();
    assert_eq!(redirect.status.as_u16(), 302);
    assert!(redirect.headers.iter().any(|header| header.value == "/new"));
}

#[tokio::test]
async fn refused_connections_are_connect_errors() {
    // Bind a free port, then close it again: nobody listens there.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let error = sender()
        .send(
            request(HttpMethod::Get, &format!("http://{address}/"), vec![]),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, TransportError::Connect(ref detail) if !detail.is_empty()),
        "{error:?}"
    );
}

#[tokio::test]
async fn invalid_header_values_are_reported() {
    let error = sender()
        .send(
            request(
                HttpMethod::Get,
                "http://127.0.0.1:1/",
                vec![Header::new("X-Bad", "line\nbreak")],
            ),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, TransportError::Other(_)), "{error:?}");
}

#[tokio::test]
async fn sends_files_and_multipart_forms() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("avatar.png");
    std::fs::write(&file, b"PNGDATA").unwrap();

    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/binary"))
        .and(body_string("PNGDATA"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/form"))
        .and(body_string_contains("name=\"title\""))
        .and(body_string_contains("Teapot"))
        .and(body_string_contains("filename=\"avatar.png\""))
        .and(body_string_contains("PNGDATA"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    let mut put = request(HttpMethod::Put, &format!("{}/binary", server.uri()), vec![]);
    put.body = Some(RequestBody::File(file.clone()));
    assert_eq!(sender().send(put, None).await.unwrap().status.as_u16(), 204);

    let mut post = request(HttpMethod::Post, &format!("{}/form", server.uri()), vec![]);
    post.body = Some(RequestBody::Multipart(vec![
        Part {
            name: "title".into(),
            content: PartContent::Text("Teapot".into()),
        },
        Part {
            name: "avatar".into(),
            content: PartContent::File(file),
        },
    ]));
    assert_eq!(
        sender().send(post, None).await.unwrap().status.as_u16(),
        201
    );
}

#[tokio::test]
async fn unreadable_files_fail_the_request() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.bin");
    for body in [
        RequestBody::File(missing.clone()),
        RequestBody::Multipart(vec![Part {
            name: "f".into(),
            content: PartContent::File(missing.clone()),
        }]),
    ] {
        let mut post = request(HttpMethod::Post, "http://127.0.0.1:9/", vec![]);
        post.body = Some(body);
        let error = sender().send(post, None).await.unwrap_err();
        assert!(
            matches!(&error, TransportError::File { path, .. } if path.ends_with("missing.bin")),
            "{error:?}"
        );
        assert_eq!(error.user_text().key, "error.file_unreadable");
    }
}

#[tokio::test]
async fn requests_without_certificate_check_use_the_unchecking_client() {
    // Only plain HTTP is available here: this shows the request goes out
    // through the second client; checking real certificates is reqwest's job.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let mut get = request(HttpMethod::Get, &server.uri(), vec![]);
    get.verify_tls = false;
    assert_eq!(sender().send(get, None).await.unwrap().status.as_u16(), 200);
}

#[tokio::test]
async fn a_manual_proxy_carries_the_requests() {
    // The mock server acts as the proxy: the request for a host that does
    // not exist arrives there.
    let proxy = MockServer::start().await;
    Mock::given(path("/via-proxy"))
        .respond_with(ResponseTemplate::new(200).set_body_string("proxied"))
        .mount(&proxy)
        .await;
    let sender = sender();
    sender
        .configure(&NetworkSettings {
            proxy: ProxySetting::Manual {
                url: proxy.uri(),
                no_proxy: String::new(),
            },
            ca_certificates: Vec::new(),
        })
        .unwrap();
    let response = sender
        .send(
            request(
                HttpMethod::Get,
                "http://api.example.invalid/via-proxy",
                vec![],
            ),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.body, b"proxied");

    // Back to direct connections: the host cannot be reached any more.
    sender
        .configure(&NetworkSettings {
            proxy: ProxySetting::None,
            ca_certificates: Vec::new(),
        })
        .unwrap();
    assert!(
        sender
            .send(
                request(
                    HttpMethod::Get,
                    "http://api.example.invalid/via-proxy",
                    vec![]
                ),
                None
            )
            .await
            .is_err()
    );
}

#[test]
fn settings_that_cannot_work_are_refused_and_change_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let garbage = temp.path().join("garbage.pem");
    std::fs::write(
        &garbage,
        "-----BEGIN CERTIFICATE-----\nnot base64\n-----END CERTIFICATE-----\n",
    )
    .unwrap();
    let empty = temp.path().join("empty.der");
    std::fs::write(&empty, [0_u8; 3]).unwrap();
    let sender = sender();
    for settings in [
        NetworkSettings {
            proxy: ProxySetting::Manual {
                url: "not a url".into(),
                no_proxy: String::new(),
            },
            ca_certificates: Vec::new(),
        },
        NetworkSettings {
            proxy: ProxySetting::System,
            ca_certificates: vec![temp.path().join("missing.pem")],
        },
        NetworkSettings {
            proxy: ProxySetting::System,
            ca_certificates: vec![garbage.clone()],
        },
        NetworkSettings {
            proxy: ProxySetting::System,
            ca_certificates: vec![empty.clone()],
        },
    ] {
        assert!(sender.configure(&settings).is_err(), "{settings:?}");
    }
    assert!(sender.configure(&NetworkSettings::default()).is_ok());
}

#[tokio::test]
async fn cookies_are_kept_across_redirects_and_requests() {
    use std::sync::{Arc, Mutex};

    use ac_domain::cookie::CookieJar;

    let server = MockServer::start().await;
    // The login sets the session on its redirect response.
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", "/home")
                .insert_header("Set-Cookie", "session=s3cret; Path=/; HttpOnly"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/home"))
        .and(header_regex("cookie", "^session=s3cret(; seen=1)?$"))
        .respond_with(ResponseTemplate::new(200).insert_header("Set-Cookie", "seen=1"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/home"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let jar = Arc::new(Mutex::new(CookieJar::new()));
    let sender = sender();
    let login = request(HttpMethod::Post, &format!("{}/login", server.uri()), vec![]);
    let response = sender.send(login, Some(jar.clone())).await.unwrap();
    assert_eq!(response.status.as_u16(), 200);
    let home = format!("{}/home", server.uri());
    assert_eq!(
        jar.lock()
            .unwrap()
            .header_for(&Url::parse(&home).unwrap(), 0),
        Some("session=s3cret; seen=1".into())
    );

    // The next request of the collection sends them on its own …
    let again = sender
        .send(request(HttpMethod::Get, &home, vec![]), Some(jar.clone()))
        .await
        .unwrap();
    assert_eq!(again.status.as_u16(), 200);
    // … without a jar (cookies off) nothing is sent …
    let without = sender
        .send(request(HttpMethod::Get, &home, vec![]), None)
        .await
        .unwrap();
    assert_eq!(without.status.as_u16(), 401);
    // … and a Cookie header of the request itself replaces the jar's.
    let own = sender
        .send(
            request(
                HttpMethod::Get,
                &home,
                vec![Header::new("Cookie", "session=other")],
            ),
            Some(jar),
        )
        .await
        .unwrap();
    assert_eq!(own.status.as_u16(), 401);
}
