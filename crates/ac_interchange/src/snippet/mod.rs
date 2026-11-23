//! A resolved request as code to send it from a program: cURL, JavaScript
//! (`fetch`, axios), Python (`requests`), Go (`net/http`) and `HTTPie`.
//!
//! - this file: [`SnippetLanguage`], [`to_snippet`] and what the generators
//!   share (string literals, the body as text).
//! - `javascript`: `fetch` and axios.
//! - `python`: `requests`.
//! - `go`: `net/http`.
//! - `httpie`: the `http` command.
//!
//! Variables and inherited auth are already applied (the caller resolves the
//! request as for sending, AWS signatures included); files are referenced by
//! path, so they are read when the code runs. HTTP Digest needs the
//! server's challenge, so it is left to the tool where it has it built in
//! (cURL, `HTTPie`, Python) and pointed out in a comment elsewhere. Switched-off certificate checks and redirects are
//! carried over where the library has a setting for them.

mod go;
mod httpie;
mod javascript;
mod python;

use ac_domain::exchange::ResolvedRequest;

use crate::curl::{DigestCredentials, to_curl};

/// What a snippet is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetLanguage {
    /// A cURL command (POSIX shell).
    Curl,
    /// JavaScript with `fetch` (browsers, Node.js 18+).
    JavaScriptFetch,
    /// JavaScript with axios.
    JavaScriptAxios,
    /// Python with `requests`.
    PythonRequests,
    /// Go with `net/http`.
    GoNetHttp,
    /// The `HTTPie` command line client.
    Httpie,
}

/// Said where a language has no Digest authentication built in.
const NO_DIGEST: &str = "Digest authentication is not built in here: send the request, then \
     answer the server's 401 challenge (WWW-Authenticate) with user name and password.";

/// The code that sends `request` in `language` (answering a Digest
/// challenge with `digest`, if given).
#[must_use]
pub fn to_snippet(
    request: &ResolvedRequest,
    language: SnippetLanguage,
    digest: Option<&DigestCredentials>,
) -> String {
    let note = |code: String, comment: &str| match digest {
        Some(_) => format!("{comment} {NO_DIGEST}\n{code}"),
        None => code,
    };
    match language {
        SnippetLanguage::Curl => to_curl(request, digest),
        SnippetLanguage::JavaScriptFetch => note(javascript::fetch(request), "//"),
        SnippetLanguage::JavaScriptAxios => note(javascript::axios(request), "//"),
        SnippetLanguage::PythonRequests => python::requests(request, digest),
        SnippetLanguage::GoNetHttp => note(go::net_http(request), "//"),
        SnippetLanguage::Httpie => httpie::command(request, digest),
    }
}

/// `text` as a double-quoted string literal with JSON escapes – valid in
/// JavaScript, Python and Go alike.
fn literal(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
}

/// A body given as bytes, as text (invalid UTF-8 replaced).
fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
#[path = "../tests/snippet.rs"]
mod tests;
