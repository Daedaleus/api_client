//! How a request authenticates. A request, folder or collection either sets
//! its own authentication or inherits it from the folder or collection
//! around it, so a whole collection can share one login.
//!
//! Values may contain `{{variables}}` – credentials belong in an
//! environment's secrets rather than in plain text in a file.

/// The authentication of a request, folder or collection.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum Auth {
    /// Use the authentication of the folder or collection around it.
    #[default]
    Inherit,
    /// Send no authentication.
    None,
    /// HTTP Basic: `Authorization: Basic base64(username:password)`.
    Basic {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// A bearer token: `Authorization: Bearer <token>`.
    Bearer {
        /// The token.
        token: String,
    },
    /// OAuth 2 with the client credentials grant: an access token is
    /// fetched from `token_url` and sent as a bearer token.
    OAuth2(OAuth2),
    /// An API key in a header or a query parameter.
    ApiKey {
        /// Header or parameter name, e.g. `X-API-Key`.
        name: String,
        /// The key.
        value: String,
        /// Where it goes.
        placement: KeyPlacement,
    },
}

/// OAuth 2 client credentials (RFC 6749, section 4.4).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct OAuth2 {
    /// Where tokens come from.
    pub token_url: String,
    /// The client's id.
    pub client_id: String,
    /// The client's secret (sent with HTTP Basic to the token URL).
    pub client_secret: String,
    /// Requested scopes, separated by spaces; empty for none.
    pub scope: String,
}

/// The secret is never shown in debug output.
impl std::fmt::Debug for OAuth2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth2")
            .field("token_url", &self.token_url)
            .field("client_id", &self.client_id)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

/// Where an API key is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyPlacement {
    /// As a header.
    #[default]
    Header,
    /// As a query parameter of the URL.
    Query,
}

/// Credentials are never shown in debug output.
impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inherit => f.write_str("Inherit"),
            Self::None => f.write_str("None"),
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .finish_non_exhaustive(),
            Self::Bearer { .. } => f.debug_struct("Bearer").finish_non_exhaustive(),
            Self::OAuth2(oauth) => oauth.fmt(f),
            Self::ApiKey {
                name, placement, ..
            } => f
                .debug_struct("ApiKey")
                .field("name", name)
                .field("placement", placement)
                .finish_non_exhaustive(),
        }
    }
}

/// The authentication that applies, given the chain from the request
/// outwards (request, inner folder … outer folder, collection): the first
/// that does not inherit. If everything inherits, nothing is sent.
#[must_use]
pub fn effective<'a>(chain: impl IntoIterator<Item = &'a Auth>) -> &'a Auth {
    chain
        .into_iter()
        .find(|auth| **auth != Auth::Inherit)
        .unwrap_or(&Auth::None)
}

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;
