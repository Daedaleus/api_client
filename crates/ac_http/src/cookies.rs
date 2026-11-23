//! The collection's cookie jar inside reqwest.
//!
//! reqwest asks a cookie provider for the `Cookie` header of every request
//! it sends and hands it the `Set-Cookie` headers of every response – also
//! for each redirect it follows, which the app could not see from outside.
//! The provider belongs to the shared client, but the jar belongs to the
//! request's collection: so the jar is put into a task-local for the time
//! one request is sent ([`with_jar`]), and the provider uses whichever jar
//! is set there. reqwest calls the provider while the send future is
//! polled, i.e. inside that scope. Without a jar (cookies off, imports,
//! token requests) nothing is sent or kept.

use std::future::Future;
use std::sync::PoisonError;
use std::time::{SystemTime, UNIX_EPOCH};

use ac_app::ports::SharedCookieJar;
use ac_domain::cookie::UnixSeconds;
use reqwest::header::HeaderValue;
use url::Url;

tokio::task_local! {
    static JAR: SharedCookieJar;
}

/// Runs `future` (one request) with `jar` as the cookie jar, or with none.
pub(crate) async fn with_jar<F: Future>(jar: Option<SharedCookieJar>, future: F) -> F::Output {
    match jar {
        Some(jar) => JAR.scope(jar, future).await,
        None => future.await,
    }
}

/// The cookie provider of every client: the jar of the request being sent.
#[derive(Debug, Default)]
pub(crate) struct JarProvider;

impl reqwest::cookie::CookieStore for JarProvider {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let _ = JAR.try_with(|jar| {
            let mut jar = jar.lock().unwrap_or_else(PoisonError::into_inner);
            let now = now();
            for header in headers {
                if let Ok(text) = header.to_str() {
                    jar.store(url, text, now);
                }
            }
        });
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        JAR.try_with(|jar| {
            jar.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .header_for(url, now())
        })
        .ok()
        .flatten()
        .and_then(|header| HeaderValue::from_str(&header).ok())
    }
}

/// The current time in the jar's terms.
fn now() -> UnixSeconds {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            UnixSeconds::try_from(elapsed.as_secs()).unwrap_or(UnixSeconds::MAX)
        })
}
