//! The cookie jars: one per open collection and one for requests outside a
//! collection, in memory only. Sending uses them (see `runs`); the cookie
//! manager lists, edits, deletes and clears their cookies.

use std::sync::PoisonError;

use ac_domain::cookie::CookieJar;

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretVault, SettingsStore, SharedCookieJar,
};
use crate::signing::unix_now;
use crate::view::{CookieKeyView, CookieView};

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
> App<H, S, V, B, R, A, Y, Z>
{
    /// The cookie jar of a collection (`None`: outside a collection),
    /// created empty when first needed.
    pub(super) fn cookie_jar(&self, collection: Option<String>) -> SharedCookieJar {
        self.inner
            .cookie_jars
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(collection)
            .or_default()
            .clone()
    }

    /// Discards the cookies of a closed collection.
    pub(super) fn forget_cookies(&self, collection_id: &str) {
        self.inner
            .cookie_jars
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&Some(collection_id.to_owned()));
    }

    pub(super) fn query_cookies(
        &self,
        collection_id: Option<&str>,
    ) -> Result<QueryResult, AppError> {
        Ok(QueryResult::Cookies {
            cookies: self.with_jar(collection_id, |_| ())?,
        })
    }

    /// Adds a cookie or replaces `original` (an edit may change name,
    /// domain or path).
    pub(super) fn save_cookie(
        &self,
        collection_id: Option<&str>,
        original: Option<&CookieKeyView>,
        cookie: &CookieView,
    ) -> Result<CommandAck, AppError> {
        let cookie = cookie.to_cookie();
        cookie.check()?;
        let cookies = self.with_jar(collection_id, |jar| {
            if let Some(original) = original {
                jar.remove(&original.domain, &original.path, &original.name);
            }
            jar.set(cookie, unix_now());
        })?;
        Ok(CommandAck::CookiesChanged { cookies })
    }

    pub(super) fn delete_cookie(
        &self,
        collection_id: Option<&str>,
        cookie: &CookieKeyView,
    ) -> Result<CommandAck, AppError> {
        let cookies = self.with_jar(collection_id, |jar| {
            jar.remove(&cookie.domain, &cookie.path, &cookie.name);
        })?;
        Ok(CommandAck::CookiesChanged { cookies })
    }

    pub(super) fn clear_cookies(
        &self,
        collection_id: Option<&str>,
    ) -> Result<CommandAck, AppError> {
        let cookies = self.with_jar(collection_id, CookieJar::clear)?;
        Ok(CommandAck::CookiesChanged { cookies })
    }

    /// Changes the jar of an open collection (or the one outside
    /// collections) and returns its cookies that have not expired.
    fn with_jar(
        &self,
        collection_id: Option<&str>,
        change: impl FnOnce(&mut CookieJar),
    ) -> Result<Vec<CookieView>, AppError> {
        if let Some(id) = collection_id {
            self.current(id)?;
        }
        let jar = self.cookie_jar(collection_id.map(str::to_owned));
        let mut jar = jar.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut jar);
        Ok(jar
            .cookies(unix_now())
            .map(CookieView::from_cookie)
            .collect())
    }
}

#[cfg(test)]
#[path = "../tests/app_cookies.rs"]
mod tests;
