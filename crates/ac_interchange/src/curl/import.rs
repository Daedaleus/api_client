//! A pasted cURL command as a request.
//!
//! Understood: the URL, `-X`, `-H`, `-d`/`--data`/`--data-raw`/
//! `--data-binary` (`@file` = file body), `--data-urlencode`, `-F`
//! (`name=@file` = file field), `-u` (Basic auth), `-A`, `-b`, `-e`, `-k`,
//! `--url`. Options that only change how cURL itself behaves (`-L`, `-s`,
//! `--compressed` …) and unknown ones are listed in [`Imported::ignored`].

use ac_domain::auth::Auth;
use ac_domain::collection::{Body, FormField, HeaderEntry, MultipartField, Request};
use ac_domain::protocol::{HttpMethod, header};
use ac_domain::tls::CertificateCheck;

/// Name of an imported request.
const NAME: &str = "cURL";

/// What a cURL command describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// The request.
    pub request: Request,
    /// Options that were not taken over, as written.
    pub ignored: Vec<String>,
}

/// Why a text is no cURL command.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CurlError {
    /// Quotes are not closed.
    #[error("unbalanced quotes")]
    Quotes,
    /// It does not start with `curl`.
    #[error("not a curl command")]
    NotCurl,
    /// No URL in it.
    #[error("no URL")]
    NoUrl,
    /// An option needs a value that is missing.
    #[error("{0} needs a value")]
    MissingValue(String),
    /// The method is not a valid token.
    #[error("invalid method {0:?}")]
    Method(String),
}

/// What the options collect before the request is put together.
#[derive(Default)]
struct Parts {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<HeaderEntry>,
    data: Vec<String>,
    data_file: Option<String>,
    urlencoded: Vec<String>,
    form: Vec<MultipartField>,
    user: Option<String>,
    insecure: bool,
    ignored: Vec<String>,
}

/// Options whose next argument is their value, but that only change how
/// cURL behaves; they are reported as ignored.
const IGNORED_WITH_VALUE: [&str; 8] = [
    "-o",
    "--output",
    "-m",
    "--max-time",
    "--connect-timeout",
    "-w",
    "--write-out",
    "--retry",
];

/// Reads a cURL command (also copied from a browser's developer tools, with
/// line continuations of bash, cmd or PowerShell).
///
/// # Errors
/// [`CurlError`] if the text is no cURL command with a URL.
pub fn from_curl(text: &str) -> Result<Imported, CurlError> {
    let joined = join_lines(text);
    let words = shell_words::split(&joined).map_err(|_| CurlError::Quotes)?;
    let mut words = words.into_iter();
    match words.next() {
        Some(first) if first == "curl" || first == "curl.exe" => {}
        _ => return Err(CurlError::NotCurl),
    }
    let mut parts = Parts::default();
    while let Some(word) = words.next() {
        let (name, inline) = split_option(&word);
        let mut value = || -> Result<String, CurlError> {
            match inline.clone() {
                Some(value) => Ok(value),
                None => words
                    .next()
                    .ok_or_else(|| CurlError::MissingValue(name.clone())),
            }
        };
        match name.as_str() {
            "-X" | "--request" => parts.method = Some(value()?),
            "-H" | "--header" => {
                if let Some(entry) = header_entry(&value()?) {
                    parts.headers.push(entry);
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-ascii" | "--data-binary" => {
                let data = value()?;
                match data.strip_prefix('@') {
                    Some(file) if name != "--data-raw" => parts.data_file = Some(file.to_owned()),
                    _ => parts.data.push(data),
                }
            }
            "--data-urlencode" => parts.urlencoded.push(value()?),
            "-F" | "--form" | "--form-string" => {
                parts
                    .form
                    .push(form_field(&value()?, name == "--form-string"));
            }
            "-u" | "--user" => parts.user = Some(value()?),
            "-A" | "--user-agent" => parts.headers.push(entry("User-Agent", &value()?)),
            "-b" | "--cookie" => parts.headers.push(entry("Cookie", &value()?)),
            "-e" | "--referer" => parts.headers.push(entry("Referer", &value()?)),
            "-k" | "--insecure" => parts.insecure = true,
            "--url" => parts.url = Some(value()?),
            option if IGNORED_WITH_VALUE.contains(&option) => {
                let value = value()?;
                parts.ignored.push(format!("{word} {value}"));
            }
            option if option.starts_with('-') => parts.ignored.push(word.clone()),
            _ => {
                if parts.url.is_none() {
                    parts.url = Some(word.clone());
                } else {
                    parts.ignored.push(word.clone());
                }
            }
        }
    }
    build(parts)
}

/// Joins continued lines (`\` in bash, `^` in cmd, `` ` `` in PowerShell).
fn join_lines(text: &str) -> String {
    let mut joined = text.replace("\r\n", "\n");
    for mark in ["\\\n", "^\n", "`\n"] {
        joined = joined.replace(mark, " ");
    }
    joined.replace('\n', " ")
}

/// `--name=value` or `-Xvalue` into name and inline value.
fn split_option(word: &str) -> (String, Option<String>) {
    if let Some(long) = word.strip_prefix("--")
        && let Some((name, value)) = long.split_once('=')
    {
        return (format!("--{name}"), Some(value.to_owned()));
    }
    let short_with_value = word.len() > 2
        && word.starts_with('-')
        && !word.starts_with("--")
        && matches!(
            &word[..2],
            "-X" | "-H" | "-d" | "-F" | "-u" | "-A" | "-b" | "-e"
        );
    if short_with_value {
        return (word[..2].to_owned(), Some(word[2..].to_owned()));
    }
    (word.to_owned(), None)
}

fn entry(name: &str, value: &str) -> HeaderEntry {
    HeaderEntry {
        name: name.to_owned(),
        value: value.to_owned(),
        enabled: true,
    }
}

fn header_entry(line: &str) -> Option<HeaderEntry> {
    let (name, value) = line.split_once(':')?;
    Some(entry(name.trim(), value.trim()))
}

fn form_field(text: &str, literal: bool) -> MultipartField {
    let (name, value) = text.split_once('=').unwrap_or((text, ""));
    let file = if literal {
        None
    } else {
        value.strip_prefix('@')
    };
    MultipartField {
        name: name.to_owned(),
        // `;type=…` and similar part options are not kept.
        value: file
            .map_or(value, |file| file.split(';').next().unwrap_or(file))
            .to_owned(),
        enabled: true,
        file: file.is_some(),
    }
}

fn build(parts: Parts) -> Result<Imported, CurlError> {
    let url = parts.url.ok_or(CurlError::NoUrl)?;
    let content_type = parts
        .headers
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(header::CONTENT_TYPE))
        .map(|entry| entry.value.to_ascii_lowercase());
    let body = if !parts.form.is_empty() {
        Body::Multipart(parts.form)
    } else if let Some(file) = parts.data_file {
        Body::Binary(file)
    } else if !parts.data.is_empty() || !parts.urlencoded.is_empty() {
        data_body(&parts.data, &parts.urlencoded, content_type.as_deref())
    } else {
        Body::None
    };
    let method = match parts.method {
        Some(method) => method,
        None if body == Body::None => HttpMethod::Get.as_str().to_owned(),
        None => HttpMethod::Post.as_str().to_owned(),
    };
    let method: HttpMethod = method.parse().map_err(|_| CurlError::Method(method))?;
    let auth = parts.user.map_or(Auth::Inherit, |user| {
        let (username, password) = user.split_once(':').unwrap_or((&user, ""));
        Auth::Basic {
            username: username.to_owned(),
            password: password.to_owned(),
        }
    });
    let mut request = Request::new(NAME);
    request.method = method;
    request.url = url;
    request.headers = parts.headers;
    request.body = body;
    request.auth = auth;
    if parts.insecure {
        request.certificate_check = CertificateCheck::Off;
    }
    Ok(Imported {
        request,
        ignored: parts.ignored,
    })
}

/// `-d` data: JSON if it says or looks so, a form if it is form data (cURL's
/// default type), text otherwise.
fn data_body(data: &[String], urlencoded: &[String], content_type: Option<&str>) -> Body {
    let mut joined = data.join("&");
    for value in urlencoded {
        if !joined.is_empty() {
            joined.push('&');
        }
        joined.push_str(&encode_pair(value));
    }
    let looks_like_json = joined.trim_start().starts_with(['{', '[']);
    match content_type {
        Some(kind) if kind.contains("json") => Body::Json(joined),
        None if looks_like_json => Body::Json(joined),
        None | Some("application/x-www-form-urlencoded") => Body::Form(
            url::form_urlencoded::parse(joined.as_bytes())
                .map(|(name, value)| FormField {
                    name: name.into_owned(),
                    value: value.into_owned(),
                    enabled: true,
                })
                .collect(),
        ),
        Some(_) => Body::Text(joined),
    }
}

/// `--data-urlencode name=value`: the value gets encoded.
fn encode_pair(text: &str) -> String {
    let (name, value) = text.split_once('=').unwrap_or(("", text));
    let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
    if name.is_empty() {
        encoded
    } else {
        format!("{name}={encoded}")
    }
}

#[cfg(test)]
#[path = "../tests/curl_import.rs"]
mod tests;
