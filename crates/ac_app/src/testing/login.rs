//! Test double for signing in in the browser.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use url::Url;

use super::lock;
use crate::ports::{BrowserLogin, LoginError, PendingLogin};

/// The port a redirect URL with port 0 gets.
pub const FAKE_PORT: u16 = 4711;

/// What the "user" does in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginAnswer {
    /// Signs in: the redirect carries this code and the right `state`.
    Approve(String),
    /// Refuses: the redirect carries this `error` and the right `state`.
    Deny(String),
    /// A redirect with this code but a wrong `state`.
    WrongState(String),
    /// The redirect carries the right `state` but no code.
    NoCode,
    /// Opening the browser or waiting fails.
    Fail(LoginError),
}

/// A [`BrowserLogin`] that answers with queued [`LoginAnswer`]s and records
/// the sign-in URLs opened. Without a queued answer it times out.
#[derive(Debug, Default)]
pub struct FakeBrowserLogin {
    answers: Arc<Mutex<VecDeque<LoginAnswer>>>,
    opened: Arc<Mutex<Vec<Url>>>,
    listen_error: Mutex<Option<LoginError>>,
}

impl FakeBrowserLogin {
    /// A login without answers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the next answer.
    #[must_use]
    pub fn answer(self, answer: LoginAnswer) -> Self {
        lock(&self.answers).push_back(answer);
        self
    }

    /// The next `listen` fails with `error`.
    #[must_use]
    pub fn fail_listen(self, error: LoginError) -> Self {
        *lock(&self.listen_error) = Some(error);
        self
    }

    /// The sign-in URLs opened so far.
    #[must_use]
    pub fn opened(&self) -> Vec<Url> {
        lock(&self.opened).clone()
    }
}

impl BrowserLogin for FakeBrowserLogin {
    type Pending = FakePendingLogin;

    fn listen(
        &self,
        redirect: &Url,
    ) -> impl Future<Output = Result<FakePendingLogin, LoginError>> + Send {
        let failure = lock(&self.listen_error).take();
        let result = if let Some(error) = failure {
            Err(error)
        } else {
            let mut redirect_uri = redirect.clone();
            if redirect.port() == Some(0) {
                // Always succeeds for http URLs.
                let _ = redirect_uri.set_port(Some(FAKE_PORT));
            }
            Ok(FakePendingLogin {
                redirect_uri,
                answers: Arc::clone(&self.answers),
                opened: Arc::clone(&self.opened),
            })
        };
        std::future::ready(result)
    }
}

/// A sign-in of [`FakeBrowserLogin`].
#[derive(Debug)]
pub struct FakePendingLogin {
    redirect_uri: Url,
    answers: Arc<Mutex<VecDeque<LoginAnswer>>>,
    opened: Arc<Mutex<Vec<Url>>>,
}

impl PendingLogin for FakePendingLogin {
    fn redirect_uri(&self) -> &Url {
        &self.redirect_uri
    }

    fn complete(
        self,
        url: Url,
    ) -> impl Future<Output = Result<Vec<(String, String)>, LoginError>> + Send {
        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map_or_default(|(_, value)| value.into_owned());
        lock(&self.opened).push(url);
        let pair = |key: &str, value: &str| (key.to_owned(), value.to_owned());
        let result = match lock(&self.answers).pop_front() {
            None => Err(LoginError::TimedOut),
            Some(LoginAnswer::Fail(error)) => Err(error),
            Some(LoginAnswer::Approve(code)) => {
                Ok(vec![pair("code", &code), pair("state", &state)])
            }
            Some(LoginAnswer::Deny(error)) => {
                Ok(vec![pair("error", &error), pair("state", &state)])
            }
            Some(LoginAnswer::WrongState(code)) => {
                Ok(vec![pair("code", &code), pair("state", "forged")])
            }
            Some(LoginAnswer::NoCode) => Ok(vec![pair("state", &state)]),
        };
        std::future::ready(result)
    }
}
