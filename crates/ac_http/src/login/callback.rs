//! Receiving the redirect at the end of a sign-in: a minimal HTTP/1.1
//! server that reads one request line and answers with a small page.

use std::time::Duration;

use ac_app::LoginError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// More than any redirect needs; longer request heads are cut off.
const MAX_HEAD: usize = 16 * 1024;
/// A connection that sends nothing (a browser's speculative pre-connect)
/// is dropped after this, so the real redirect is not kept waiting.
const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// The query parameter that marks a refused sign-in.
const ERROR_PARAM: &str = "error";

/// The page the browser shows after the redirect. Only a symbol, no words:
/// the listener runs below the user interface and does not know its
/// language, and the app itself reports what happened.
fn page(success: bool) -> String {
    let symbol = if success { "&#10003;" } else { "&#10007;" };
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{symbol}</title>\
         <body style=\"font:64px system-ui,sans-serif;text-align:center;margin-top:20vh\">\
         {symbol}</body>"
    )
}

/// Waits for a request to `path` and returns its query parameters; other
/// requests (such as `/favicon.ico`) get a 404 and waiting goes on.
pub(super) async fn receive(
    listener: &TcpListener,
    path: &str,
) -> Result<Vec<(String, String)>, LoginError> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|error| LoginError::Redirect(error.to_string()))?;
        let Ok(Some(target)) = tokio::time::timeout(READ_TIMEOUT, request_target(&mut stream))
            .await
            .unwrap_or(Ok(None))
        else {
            continue;
        };
        let (target_path, query) = target.split_once('?').unwrap_or((&target, ""));
        if target_path != path {
            let _ = respond(&mut stream, "404 Not Found", "").await;
            continue;
        }
        let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
        let success = !params.iter().any(|(key, _)| key == ERROR_PARAM);
        // The redirect counts even if the page does not reach the browser.
        let _ = respond(&mut stream, "200 OK", &page(success)).await;
        return Ok(params);
    }
}

/// The target of a `GET` request (`/callback?code=…`); `None` for anything
/// else.
async fn request_target(stream: &mut TcpStream) -> std::io::Result<Option<String>> {
    let mut head = Vec::new();
    let mut buffer = [0; 2048];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < MAX_HEAD {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&buffer[..read]);
    }
    let head = String::from_utf8_lossy(&head);
    let mut parts = head.lines().next().unwrap_or_default().split(' ');
    Ok(match (parts.next(), parts.next()) {
        (Some("GET"), Some(target)) => Some(target.to_owned()),
        _ => None,
    })
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\n\
         content-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}
