//! Cookies a server sets, kept per collection and sent back like a browser
//! does (RFC 6265): matching domain and path, not expired, `Secure` ones
//! only over HTTPS (or to the own machine).
//!
//! - [`parse`]: one `Set-Cookie` header.
//! - [`date`]: the lenient date format of `Expires`.
//!
//! Pure rules: the current time is passed in (seconds since 1970), so
//! nothing here reads the clock. Not implemented: the public suffix list –
//! a `Domain` attribute only needs a dot and to match the host, so
//! `Domain=co.uk` is accepted from `shop.co.uk`.

mod date;
mod parse;

pub use date::parse_cookie_date;

use url::Url;

/// Seconds since 1970-01-01 UTC.
pub type UnixSeconds = i64;

/// One cookie in the jar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    /// Name.
    pub name: String,
    /// Value as sent.
    pub value: String,
    /// The host it belongs to, lower case and without a leading dot.
    pub domain: String,
    /// Only for exactly `domain` (no `Domain` attribute), else also for
    /// its subdomains.
    pub host_only: bool,
    /// Sent for this path and below.
    pub path: String,
    /// When it expires; `None` for a session cookie (kept until the app
    /// ends or the jar is cleared).
    pub expires: Option<UnixSeconds>,
    /// Only over secure connections.
    pub secure: bool,
    /// Not for scripts in a browser (informational here).
    pub http_only: bool,
}

impl Cookie {
    fn is_expired(&self, now: UnixSeconds) -> bool {
        self.expires.is_some_and(|expires| expires <= now)
    }

    fn same_slot(&self, other: &Self) -> bool {
        self.name == other.name && self.domain == other.domain && self.path == other.path
    }

    fn matches(&self, url: &Url, now: UnixSeconds) -> bool {
        let Some(host) = host_of(url) else {
            return false;
        };
        let domain_ok = if self.host_only {
            host == self.domain
        } else {
            domain_matches(&host, &self.domain)
        };
        domain_ok
            && path_matches(url.path(), &self.path)
            && (!self.secure || is_secure(url))
            && !self.is_expired(now)
    }
}

/// The cookies of one collection, in the order they were first set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
}

impl CookieJar {
    /// An empty jar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes over a `Set-Cookie` header received from `url`. Invalid ones
    /// and ones for another domain are ignored; an expired one removes the
    /// cookie it names.
    pub fn store(&mut self, url: &Url, set_cookie: &str, now: UnixSeconds) {
        if let Some(cookie) = parse::cookie_from(url, set_cookie, now) {
            self.set(cookie, now);
        }
    }

    /// Adds `cookie` or replaces the one with the same name, domain and
    /// path; an expired one only removes that.
    pub fn set(&mut self, cookie: Cookie, now: UnixSeconds) {
        let existing = self.cookies.iter().position(|c| c.same_slot(&cookie));
        match (existing, cookie.is_expired(now)) {
            (Some(index), true) => {
                self.cookies.remove(index);
            }
            (Some(index), false) => self.cookies[index] = cookie,
            (None, true) => {}
            (None, false) => self.cookies.push(cookie),
        }
    }

    /// The `Cookie` header for a request to `url`: the matching cookies,
    /// longer paths first; `None` if there are none.
    #[must_use]
    pub fn header_for(&self, url: &Url, now: UnixSeconds) -> Option<String> {
        let mut matching: Vec<&Cookie> = self
            .cookies
            .iter()
            .filter(|cookie| cookie.matches(url, now))
            .collect();
        if matching.is_empty() {
            return None;
        }
        matching.sort_by_key(|cookie| std::cmp::Reverse(cookie.path.len()));
        Some(
            matching
                .iter()
                .map(|cookie| format!("{}={}", cookie.name, cookie.value))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    /// The cookies that have not expired.
    pub fn cookies(&self, now: UnixSeconds) -> impl Iterator<Item = &Cookie> {
        self.cookies
            .iter()
            .filter(move |cookie| !cookie.is_expired(now))
    }

    /// Removes the cookie with this name, domain and path; whether there
    /// was one.
    pub fn remove(&mut self, domain: &str, path: &str, name: &str) -> bool {
        let before = self.cookies.len();
        self.cookies
            .retain(|c| !(c.domain == domain && c.path == path && c.name == name));
        self.cookies.len() != before
    }

    /// Removes all cookies.
    pub fn clear(&mut self) {
        self.cookies.clear();
    }
}

/// The host of `url`, lower case (`[::1]` for IPv6).
fn host_of(url: &Url) -> Option<String> {
    url.host_str().map(str::to_ascii_lowercase)
}

/// `host` is `domain` or one of its subdomains (never for IP addresses).
fn domain_matches(host: &str, domain: &str) -> bool {
    host == domain
        || (host.ends_with(domain)
            && host[..host.len() - domain.len()].ends_with('.')
            && !is_ip(host))
}

fn is_ip(host: &str) -> bool {
    host.starts_with('[') || host.parse::<std::net::Ipv4Addr>().is_ok()
}

/// RFC 6265 section 5.1.4.
fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/') || request_path[cookie_path.len()..].starts_with('/')))
}

/// Schemes of secure connections.
const SECURE_SCHEMES: [&str; 2] = ["https", "wss"];
/// The host name of the own machine.
const LOCALHOST: &str = "localhost";

/// HTTPS, or the own machine (browsers treat it as secure too).
fn is_secure(url: &Url) -> bool {
    SECURE_SCHEMES.contains(&url.scheme())
        || matches!(
            url.host(),
            Some(
                url::Host::Domain(LOCALHOST)
                    | url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST)
                    | url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST)
            )
        )
}

#[cfg(test)]
#[path = "../tests/cookie.rs"]
mod tests;
