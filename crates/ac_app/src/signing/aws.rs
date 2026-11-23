//! AWS Signature Version 4 (header-based): the canonical request of the
//! request as sent, signed with a key derived from the secret, date, region
//! and service. Bodies the client streams (a file, multipart) are sent as
//! `UNSIGNED-PAYLOAD`, which S3 accepts; other services need a body that
//! is part of the request text.

use ac_domain::exchange::{Header, RequestBody, ResolvedRequest};
use ac_domain::protocol::header;
use hmac::{Hmac, Mac};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

use url::Url;

use super::hex;

/// The algorithm, as written in the signature.
const ALGORITHM: &str = "AWS4-HMAC-SHA256";
/// Last part of the credential scope.
const TERMINATOR: &str = "aws4_request";
/// The payload hash of a body that is not signed.
const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";
/// S3 encodes paths once (others twice) and wants the payload hash header.
const S3: &str = "s3";

/// Headers the signature adds, in lower case.
mod amz {
    pub const DATE: &str = "x-amz-date";
    pub const CONTENT_SHA256: &str = "x-amz-content-sha256";
    pub const SECURITY_TOKEN: &str = "x-amz-security-token";
}

/// Credentials with variables filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    /// Empty if none.
    pub session_token: String,
    pub region: String,
    pub service: String,
}

/// Signs `request` at `now` (seconds since 1970): adds `X-Amz-Date`, the
/// session token and payload hash if needed, and `Authorization` (which it
/// replaces).
pub(crate) fn sign(request: &mut ResolvedRequest, credentials: &AwsCredentials, now: i64) {
    let (date, timestamp) = amz_date(now);
    let s3 = credentials.service == S3;
    let payload_hash = match &request.body {
        None => hex(&Sha256::digest(b"")),
        Some(RequestBody::Bytes(bytes)) => hex(&Sha256::digest(bytes)),
        Some(RequestBody::File(_) | RequestBody::Multipart(_)) => UNSIGNED_PAYLOAD.to_owned(),
    };
    request
        .headers
        .retain(|h| !h.name.eq_ignore_ascii_case(header::AUTHORIZATION));
    set_header(&mut request.headers, amz::DATE, &timestamp);
    if s3 || payload_hash == UNSIGNED_PAYLOAD {
        set_header(&mut request.headers, amz::CONTENT_SHA256, &payload_hash);
    }
    if !credentials.session_token.is_empty() {
        set_header(
            &mut request.headers,
            amz::SECURITY_TOKEN,
            &credentials.session_token,
        );
    }

    let (canonical_headers, signed_headers) = canonical_headers(&request.url, &request.headers);
    let canonical_request = [
        request.method.as_str(),
        &canonical_path(&request.url, s3),
        &canonical_query(&request.url),
        &canonical_headers,
        &signed_headers,
        &payload_hash,
    ]
    .join("\n");
    let scope = format!(
        "{date}/{}/{}/{TERMINATOR}",
        credentials.region, credentials.service
    );
    let string_to_sign = format!(
        "{ALGORITHM}\n{timestamp}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let key = [
        date.as_str(),
        &credentials.region,
        &credentials.service,
        TERMINATOR,
    ]
    .iter()
    .fold(
        format!("AWS4{}", credentials.secret_access_key).into_bytes(),
        |key, part| hmac(&key, part.as_bytes()),
    );
    let signature = hex(&hmac(&key, string_to_sign.as_bytes()));
    request.headers.push(Header::new(
        "Authorization",
        format!(
            "{ALGORITHM} Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            credentials.access_key_id
        ),
    ));
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Replaces a header (any case) or adds it.
fn set_header(headers: &mut Vec<Header>, name: &str, value: &str) {
    headers.retain(|h| !h.name.eq_ignore_ascii_case(name));
    headers.push(Header::new(name, value));
}

/// `host` (as the client sends it) and the request's headers: lower-case
/// names sorted, values trimmed with inner spaces collapsed, repeated
/// headers joined by commas; and the list of their names.
fn canonical_headers(url: &Url, headers: &[Header]) -> (String, String) {
    let mut entries: Vec<(String, String)> = Vec::new();
    if !headers
        .iter()
        .any(|h| h.name.eq_ignore_ascii_case(header::HOST))
    {
        entries.push((header::HOST.to_owned(), host_of(url)));
    }
    for h in headers {
        let name = h.name.to_ascii_lowercase();
        let value = h.value.split_whitespace().collect::<Vec<_>>().join(" ");
        match entries.iter_mut().find(|(existing, _)| *existing == name) {
            Some((_, joined)) => {
                joined.push(',');
                joined.push_str(&value);
            }
            None => entries.push((name, value)),
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let canonical = entries
        .iter()
        .fold(String::new(), |mut text, (name, value)| {
            let _ = writeln!(text, "{name}:{value}");
            text
        });
    let signed = entries
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");
    (canonical, signed)
}

/// The `Host` header the client sends: the port only if not the default.
fn host_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    }
}

/// Each path segment URI-encoded (twice except for S3, as AWS requires).
fn canonical_path(url: &Url, s3: bool) -> String {
    let path = url.path();
    if path.is_empty() {
        return "/".to_owned();
    }
    path.split('/')
        .map(|segment| {
            let decoded = percent_decode(segment);
            let once = uri_encode(&decoded);
            if s3 {
                once
            } else {
                uri_encode(once.as_bytes())
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Query parameters URI-encoded and sorted by name, then value.
fn canonical_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (
                uri_encode(&percent_decode(name)),
                uri_encode(&percent_decode(value)),
            )
        })
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// `%XX` sequences as bytes (no `+` for space: AWS reads queries literally).
fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && let Some(byte) = bytes
                .get(index + 1..index + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    decoded
}

/// AWS's URI encoding: everything but `A-Z a-z 0-9 - _ . ~` as `%XX`.
fn uri_encode(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, &byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            text.push(char::from(byte));
        } else {
            let _ = std::fmt::Write::write_fmt(&mut text, format_args!("%{byte:02X}"));
        }
        text
    })
}

/// `20150830` and `20150830T123600Z` for `now` (seconds since 1970, UTC).
pub(crate) fn amz_date(now: i64) -> (String, String) {
    let days = now.div_euclid(86_400);
    let seconds = now.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let date = format!("{year:04}{month:02}{day:02}");
    let timestamp = format!(
        "{date}T{:02}{:02}{:02}Z",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    );
    (date, timestamp)
}

/// Year, month and day of a day count since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
