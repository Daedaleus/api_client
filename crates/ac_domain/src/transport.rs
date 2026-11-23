//! How a request travels: whether redirects are followed, how long it may
//! take and whether the collection's cookies are used. Like the certificate
//! check, each setting is inherited from the folders around a request and
//! the collection; nothing set means the defaults (follow redirects,
//! [`DEFAULT_TIMEOUT_SECONDS`], cookies on).

use std::time::Duration;

/// Seconds a whole request may take (including reading the body) unless a
/// request, folder or the collection says otherwise.
pub const DEFAULT_TIMEOUT_SECONDS: u32 = 30;

/// Whether redirects (3xx with `Location`) are followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Redirects {
    /// As the folder or collection around it (followed, if nothing says else).
    #[default]
    Inherit,
    /// Follow them (up to the HTTP client's limit) and show the last response.
    Follow,
    /// Show the redirect response itself.
    Stop,
}

/// Whether the cookie jar is used: its cookies sent, received ones kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cookies {
    /// As the folder or collection around it (on, if nothing says else).
    #[default]
    Inherit,
    /// Use the jar.
    On,
    /// Neither send nor keep cookies.
    Off,
}

/// The transport settings of a request, folder or collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransportSettings {
    /// Redirects.
    pub redirects: Redirects,
    /// Time limit in seconds; `None` inherits.
    pub timeout_seconds: Option<u32>,
    /// The cookie jar.
    pub cookies: Cookies,
}

/// What applies to one request once inheritance is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveTransport {
    /// Follow redirects.
    pub follow_redirects: bool,
    /// Time allowed for the whole request.
    pub timeout: Duration,
    /// Send the jar's cookies and keep the ones received.
    pub cookies: bool,
}

impl Default for EffectiveTransport {
    fn default() -> Self {
        Self {
            follow_redirects: true,
            timeout: Duration::from_secs(u64::from(DEFAULT_TIMEOUT_SECONDS)),
            cookies: true,
        }
    }
}

/// The settings that apply, given the chain from the request outwards
/// (request, inner folder … outer folder, collection): for each setting the
/// first one that does not inherit decides, else the default.
#[must_use]
pub fn effective<'a>(chain: impl IntoIterator<Item = &'a TransportSettings>) -> EffectiveTransport {
    let mut redirects = None;
    let mut timeout = None;
    let mut cookies = None;
    for settings in chain {
        if cookies.is_none() && settings.cookies != Cookies::Inherit {
            cookies = Some(settings.cookies == Cookies::On);
        }
        if redirects.is_none() && settings.redirects != Redirects::Inherit {
            redirects = Some(settings.redirects);
        }
        if timeout.is_none() {
            timeout = settings.timeout_seconds;
        }
    }
    let defaults = EffectiveTransport::default();
    EffectiveTransport {
        follow_redirects: redirects.map_or(defaults.follow_redirects, |r| r == Redirects::Follow),
        timeout: timeout.map_or(defaults.timeout, |seconds| {
            Duration::from_secs(u64::from(seconds))
        }),
        cookies: cookies.unwrap_or(defaults.cookies),
    }
}

#[cfg(test)]
#[path = "tests/transport.rs"]
mod tests;
