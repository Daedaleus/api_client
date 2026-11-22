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
//! request as for sending); files are referenced by path, so they are read
//! when the code runs. Switched-off certificate checks and redirects are
//! carried over where the library has a setting for them.

mod go;
mod httpie;
mod javascript;
mod python;

use ac_domain::exchange::ResolvedRequest;

use crate::curl::to_curl;

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

/// The code that sends `request` in `language`.
#[must_use]
pub fn to_snippet(request: &ResolvedRequest, language: SnippetLanguage) -> String {
    match language {
        SnippetLanguage::Curl => to_curl(request),
        SnippetLanguage::JavaScriptFetch => javascript::fetch(request),
        SnippetLanguage::JavaScriptAxios => javascript::axios(request),
        SnippetLanguage::PythonRequests => python::requests(request),
        SnippetLanguage::GoNetHttp => go::net_http(request),
        SnippetLanguage::Httpie => httpie::command(request),
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
