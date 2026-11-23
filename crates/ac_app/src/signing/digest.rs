//! HTTP Digest (RFC 7616): reading the server's challenge from a 401
//! response and computing the `Authorization` header that answers it.
//! Algorithms MD5, SHA-256 and their `-sess` variants; `qop=auth` or no
//! `qop` (RFC 2069). `auth-int` and hashed user names are not supported.

use ac_domain::protocol::auth_scheme;
use md5::Md5;
use sha2::{Digest as _, Sha256};

use super::hex;

/// The `qop` this program answers with.
const QOP_AUTH: &str = "auth";

/// A hash algorithm of Digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Algorithm {
    Md5,
    Sha256,
}

impl Algorithm {
    fn hash(self, text: &str) -> String {
        match self {
            Self::Md5 => hex(&Md5::digest(text.as_bytes())),
            Self::Sha256 => hex(&Sha256::digest(text.as_bytes())),
        }
    }
}

/// What the server asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Challenge {
    pub realm: String,
    pub nonce: String,
    pub opaque: Option<String>,
    pub algorithm: Algorithm,
    /// `-sess`: the first hash also covers the nonces.
    pub session: bool,
    /// `qop=auth` offered (else the old form without `qop`).
    pub qop_auth: bool,
    /// The algorithm as the server wrote it (sent back as is).
    pub algorithm_name: Option<String>,
}

/// The best Digest challenge in `WWW-Authenticate` values (SHA-256 before
/// MD5); `None` if there is none this program can answer.
pub(crate) fn best_challenge<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<Challenge> {
    values
        .into_iter()
        .flat_map(split_challenges)
        .filter_map(|challenge| parse(&challenge))
        .max_by_key(|challenge| challenge.algorithm == Algorithm::Sha256)
}

/// One header value may hold several challenges (`Digest …, Basic …`):
/// the parts that start with a scheme name.
fn split_challenges(value: &str) -> Vec<String> {
    let mut challenges: Vec<String> = Vec::new();
    for part in split_params(value) {
        let starts_scheme = part
            .split_once(' ')
            .is_some_and(|(scheme, rest)| !scheme.contains('=') && rest.contains('='))
            || (!part.contains('=') && !part.is_empty());
        match challenges.last_mut() {
            Some(current) if !starts_scheme => {
                current.push_str(", ");
                current.push_str(&part);
            }
            _ => challenges.push(part),
        }
    }
    challenges
}

/// A `Digest key=value, …` challenge.
fn parse(challenge: &str) -> Option<Challenge> {
    let (scheme, params) = challenge.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case(auth_scheme::DIGEST) {
        return None;
    }
    let mut realm = None;
    let mut nonce = None;
    let mut opaque = None;
    let mut algorithm_name = None;
    let mut qop = None;
    for param in split_params(params) {
        let Some((key, value)) = param.split_once('=') else {
            continue;
        };
        let value = unquote(value.trim());
        match key.trim().to_ascii_lowercase().as_str() {
            "realm" => realm = Some(value),
            "nonce" => nonce = Some(value),
            "opaque" => opaque = Some(value),
            "algorithm" => algorithm_name = Some(value),
            "qop" => qop = Some(value),
            _ => {}
        }
    }
    let name = algorithm_name.clone().unwrap_or_else(|| "MD5".to_owned());
    let upper = name.to_ascii_uppercase();
    let (base, session) = match upper.strip_suffix("-SESS") {
        Some(base) => (base.to_owned(), true),
        None => (upper, false),
    };
    let algorithm = match base.as_str() {
        "MD5" => Algorithm::Md5,
        "SHA-256" => Algorithm::Sha256,
        _ => return None,
    };
    let qop_auth = match &qop {
        None => false,
        Some(options) => {
            if !options.split(',').any(|option| option.trim() == QOP_AUTH) {
                return None;
            }
            true
        }
    };
    Some(Challenge {
        realm: realm?,
        nonce: nonce?,
        opaque,
        algorithm,
        session,
        qop_auth,
        algorithm_name,
    })
}

/// Splits at commas outside quotes.
fn split_params(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in text.chars() {
        match c {
            _ if escaped => {
                current.push(c);
                escaped = false;
            }
            '\\' if quoted => {
                current.push(c);
                escaped = true;
            }
            '"' => {
                current.push(c);
                quoted = !quoted;
            }
            ',' if !quoted => parts.push(std::mem::take(&mut current).trim().to_owned()),
            _ => current.push(c),
        }
    }
    parts.push(current.trim().to_owned());
    parts.retain(|part| !part.is_empty());
    parts
}

fn unquote(value: &str) -> String {
    let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
        return value.to_owned();
    };
    let mut text = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                text.push(next);
            }
        } else {
            text.push(c);
        }
    }
    text
}

/// A quoted string for the header.
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// What the `Authorization` answer is computed from.
pub(crate) struct Answer<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub method: &'a str,
    /// The request target: path and query.
    pub uri: &'a str,
    /// The client's nonce (random per request).
    pub cnonce: &'a str,
    /// How often this nonce was used (1 for the first request).
    pub nonce_count: u32,
}

/// The `Authorization` header value answering `challenge`.
pub(crate) fn authorization(challenge: &Challenge, answer: &Answer<'_>) -> String {
    let hash = |text: &str| challenge.algorithm.hash(text);
    let nc = format!("{:08x}", answer.nonce_count);
    let mut ha1 = hash(&format!(
        "{}:{}:{}",
        answer.username, challenge.realm, answer.password
    ));
    if challenge.session {
        ha1 = hash(&format!("{ha1}:{}:{}", challenge.nonce, answer.cnonce));
    }
    let ha2 = hash(&format!("{}:{}", answer.method, answer.uri));
    let response = if challenge.qop_auth {
        hash(&format!(
            "{ha1}:{}:{nc}:{}:{QOP_AUTH}:{ha2}",
            challenge.nonce, answer.cnonce
        ))
    } else {
        hash(&format!("{ha1}:{}:{ha2}", challenge.nonce))
    };
    let mut parts = vec![
        format!("username={}", quote(answer.username)),
        format!("realm={}", quote(&challenge.realm)),
        format!("uri={}", quote(answer.uri)),
    ];
    if let Some(name) = &challenge.algorithm_name {
        parts.push(format!("algorithm={name}"));
    }
    parts.push(format!("nonce={}", quote(&challenge.nonce)));
    if challenge.qop_auth {
        parts.push(format!("nc={nc}"));
        parts.push(format!("cnonce={}", quote(answer.cnonce)));
        parts.push(format!("qop={QOP_AUTH}"));
    }
    parts.push(format!("response={}", quote(&response)));
    if let Some(opaque) = &challenge.opaque {
        parts.push(format!("opaque={}", quote(opaque)));
    }
    format!("{} {}", auth_scheme::DIGEST, parts.join(", "))
}

/// A fresh client nonce: 16 random bytes as hex.
pub(crate) fn new_cnonce() -> String {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("the operating system's random source works");
    hex(&bytes)
}
