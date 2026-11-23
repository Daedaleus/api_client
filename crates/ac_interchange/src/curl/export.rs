//! A resolved request as a cURL command.

use std::fmt::Write as _;
use std::path::Path;

use ac_domain::exchange::{PartContent, RequestBody, ResolvedRequest};
use ac_domain::protocol::HttpMethod;

/// Continues the command on the next line.
const NEXT_LINE: &str = " \\\n  ";

/// User name and password of HTTP Digest: an export cannot answer the
/// server's challenge itself, so the tool that runs the code does (cURL's
/// `--digest`, `HTTPie`'s `--auth-type=digest`, Python's
/// `HTTPDigestAuth`).
#[derive(Clone, PartialEq, Eq)]
pub struct DigestCredentials {
    /// User name.
    pub username: String,
    /// Password.
    pub password: String,
}

impl std::fmt::Debug for DigestCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DigestCredentials")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// The command that sends `request` with cURL (answering a Digest
/// challenge with `digest`, if given). Values are quoted for a POSIX shell;
/// files are referenced with `@path`, so they are read when the command
/// runs.
#[must_use]
pub fn to_curl(request: &ResolvedRequest, digest: Option<&DigestCredentials>) -> String {
    let mut command = String::from("curl");
    let sends_body = request.body.is_some();
    // cURL infers POST from a body and GET without one.
    let implied = if sends_body {
        HttpMethod::Post
    } else {
        HttpMethod::Get
    };
    if request.method != implied {
        option(&mut command, "--request", request.method.as_str());
    }
    let _ = write!(command, "{NEXT_LINE}{}", quote(request.url.as_str()));
    for header in &request.headers {
        option(
            &mut command,
            "--header",
            &format!("{}: {}", header.name, header.value),
        );
    }
    match &request.body {
        None => {}
        Some(RequestBody::Bytes(bytes)) => {
            option(&mut command, "--data-raw", &String::from_utf8_lossy(bytes));
        }
        Some(RequestBody::File(path)) => {
            option(&mut command, "--data-binary", &at_file(path));
        }
        Some(RequestBody::Multipart(parts)) => {
            for part in parts {
                let value = match &part.content {
                    PartContent::Text(text) => text.clone(),
                    PartContent::File(path) => at_file(path),
                };
                option(&mut command, "--form", &format!("{}={value}", part.name));
            }
        }
    }
    if let Some(digest) = digest {
        let _ = write!(command, "{NEXT_LINE}--digest");
        option(
            &mut command,
            "--user",
            &format!("{}:{}", digest.username, digest.password),
        );
    }
    if !request.verify_tls {
        let _ = write!(command, "{NEXT_LINE}--insecure");
    }
    command
}

fn option(command: &mut String, name: &str, value: &str) {
    let _ = write!(command, "{NEXT_LINE}{name} {}", quote(value));
}

fn at_file(path: &Path) -> String {
    format!("@{}", path.display())
}

/// `text` in single quotes for a POSIX shell (a `'` becomes `'\''`).
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(test)]
#[path = "../tests/curl_export.rs"]
mod tests;
