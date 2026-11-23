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
    /// OAuth 2: an access token is fetched from the token URL and sent as
    /// a bearer token.
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
    /// HTTP Digest (RFC 7616: MD5 or SHA-256, `qop=auth`): the server's
    /// challenge (a 401 response) is answered when sending.
    Digest {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// AWS Signature Version 4: the request is signed just before it is
    /// sent.
    AwsSigV4(AwsSigV4),
}

/// OAuth 2 (RFC 6749).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct OAuth2 {
    /// How the token is obtained.
    pub grant: OAuth2Grant,
    /// Where tokens come from.
    pub token_url: String,
    /// Where the user signs in (authorization code only).
    pub auth_url: String,
    /// Where the browser returns to after signing in: a loopback URL such
    /// as `http://127.0.0.1:8080/callback`; empty for any free port
    /// (authorization code only).
    pub redirect_url: String,
    /// The client's id.
    pub client_id: String,
    /// The client's secret (sent with HTTP Basic to the token URL); may be
    /// empty for public clients signing in with PKCE.
    pub client_secret: String,
    /// Requested scopes, separated by spaces; empty for none.
    pub scope: String,
}

/// How an OAuth 2 access token is obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OAuth2Grant {
    /// The client authenticates as itself, without a user (RFC 6749,
    /// section 4.4).
    #[default]
    ClientCredentials,
    /// The user signs in in the browser; secured with PKCE (RFC 6749
    /// section 4.1, RFC 7636, RFC 8252).
    AuthorizationCode,
}

/// The secret is never shown in debug output.
impl std::fmt::Debug for OAuth2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth2")
            .field("grant", &self.grant)
            .field("token_url", &self.token_url)
            .field("auth_url", &self.auth_url)
            .field("redirect_url", &self.redirect_url)
            .field("client_id", &self.client_id)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

/// Whether `url` can be an OAuth 2 redirect of a native app: `http` on the
/// loopback address, the only kind it can receive without a server
/// (RFC 8252, section 7.3).
#[must_use]
pub fn is_loopback_redirect(url: &url::Url) -> bool {
    let loopback = match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };
    url.scheme() == "http" && loopback
}

/// Credentials of AWS Signature Version 4.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct AwsSigV4 {
    /// Access key id, e.g. `AKIA…`.
    pub access_key_id: String,
    /// Secret access key (best a `{{variable}}` of an environment secret).
    pub secret_access_key: String,
    /// Session token of temporary credentials; empty if none.
    pub session_token: String,
    /// Region, e.g. `eu-central-1`.
    pub region: String,
    /// Service, e.g. `execute-api` or `s3`.
    pub service: String,
}

impl std::fmt::Debug for AwsSigV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsSigV4")
            .field("access_key_id", &self.access_key_id)
            .field("region", &self.region)
            .field("service", &self.service)
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
            Self::Digest { username, .. } => f
                .debug_struct("Digest")
                .field("username", username)
                .finish_non_exhaustive(),
            Self::AwsSigV4(aws) => aws.fmt(f),
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
