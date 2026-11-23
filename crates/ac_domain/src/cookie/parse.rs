//! One `Set-Cookie` header as a [`Cookie`] (RFC 6265 section 5.2).

use url::Url;

use super::{
    Cookie, UnixSeconds, date::parse_cookie_date, domain_matches, host_of, is_ip, is_secure,
};

/// Attribute names, compared without case.
mod attribute {
    pub const EXPIRES: &str = "expires";
    pub const MAX_AGE: &str = "max-age";
    pub const DOMAIN: &str = "domain";
    pub const PATH: &str = "path";
    pub const SECURE: &str = "secure";
    pub const HTTP_ONLY: &str = "httponly";
}

/// The cookie `set_cookie` sets when received from `url`; `None` if it is
/// invalid or not allowed for that host.
pub(super) fn cookie_from(url: &Url, set_cookie: &str, now: UnixSeconds) -> Option<Cookie> {
    let host = host_of(url)?;
    let mut parts = set_cookie.split(';');
    let (name, value) = parts.next()?.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let mut cookie = Cookie {
        name: name.to_owned(),
        value: value.trim().to_owned(),
        domain: host.clone(),
        host_only: true,
        path: default_path(url.path()),
        expires: None,
        secure: false,
        http_only: false,
    };
    let mut max_age = None;
    let mut expires = None;
    for attribute in parts {
        let (key, value) = attribute.split_once('=').unwrap_or((attribute, ""));
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            attribute::EXPIRES => expires = parse_cookie_date(value).or(expires),
            attribute::MAX_AGE => {
                if let Ok(seconds) = value.parse::<i64>() {
                    max_age = Some(if seconds <= 0 {
                        UnixSeconds::MIN
                    } else {
                        now.saturating_add(seconds)
                    });
                }
            }
            attribute::DOMAIN if !value.is_empty() => {
                let domain = value.trim_start_matches('.').to_ascii_lowercase();
                // Only the host itself or a parent with a dot – never a
                // bare top-level domain or another site; IPs only exactly.
                let allowed = domain == host
                    || (domain.contains('.') && !is_ip(&host) && domain_matches(&host, &domain));
                if !allowed {
                    return None;
                }
                cookie.host_only = false;
                cookie.domain = domain;
            }
            attribute::PATH if value.starts_with('/') => value.clone_into(&mut cookie.path),
            attribute::SECURE => cookie.secure = true,
            attribute::HTTP_ONLY => cookie.http_only = true,
            _ => {}
        }
    }
    // Max-Age wins over Expires.
    cookie.expires = max_age.or(expires);
    // A secure cookie only comes from a secure origin (RFC 6265bis).
    if cookie.secure && !is_secure(url) {
        return None;
    }
    Some(cookie)
}

/// The folder of the request path: `/a/b/c` → `/a/b`, `/a` → `/`.
fn default_path(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(last) => path[..last].to_owned(),
    }
}
