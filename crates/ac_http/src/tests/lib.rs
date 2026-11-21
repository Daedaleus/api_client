use ac_domain::protocol::HttpMethod;
use url::Url;
use wiremock::matchers::{body_string, body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn request(method: HttpMethod, url: &str, headers: Vec<Header>) -> ResolvedRequest {
    ResolvedRequest {
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
    let response = sender().send(post).await.unwrap();
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
        .send(request(
            HttpMethod::Put,
            &url,
            vec![Header::new("X-Test", "yes")],
        ))
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
        .send(request(HttpMethod::Get, &server.uri(), vec![]))
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
        .send(request(method, &server.uri(), vec![]))
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

    let sender = ReqwestSender::new(Duration::from_millis(100)).unwrap();
    let error = sender
        .send(request(HttpMethod::Get, &server.uri(), vec![]))
        .await
        .unwrap_err();
    assert_eq!(error, TransportError::Timeout);
}

#[tokio::test]
async fn refused_connections_are_connect_errors() {
    // Bind a free port, then close it again: nobody listens there.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let error = sender()
        .send(request(
            HttpMethod::Get,
            &format!("http://{address}/"),
            vec![],
        ))
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
        .send(request(
            HttpMethod::Get,
            "http://127.0.0.1:1/",
            vec![Header::new("X-Bad", "line\nbreak")],
        ))
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
    assert_eq!(sender().send(put).await.unwrap().status.as_u16(), 204);

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
    assert_eq!(sender().send(post).await.unwrap().status.as_u16(), 201);
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
        let error = sender().send(post).await.unwrap_err();
        assert!(
            matches!(&error, TransportError::File { path, .. } if path.ends_with("missing.bin")),
            "{error:?}"
        );
        assert_eq!(error.user_text().key, "error.file_unreadable");
    }
}
