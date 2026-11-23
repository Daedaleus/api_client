//! Authentication and the certificate check as the editors show and return
//! them.

use ac_domain::auth::{Auth, AwsSigV4, KeyPlacement, OAuth2, OAuth2Grant};
use ac_domain::tls::CertificateCheck;
use serde::{Deserialize, Serialize};

/// The authentication of a request, folder or collection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AuthDraft {
    /// Use that of the folder or collection around it.
    #[default]
    Inherit,
    /// None.
    None,
    /// HTTP Basic.
    Basic {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// Bearer token.
    Bearer {
        /// The token.
        token: String,
    },
    /// OAuth 2.
    #[serde(rename = "oauth2")]
    OAuth2 {
        /// How the token is obtained.
        grant: OAuth2GrantView,
        /// Where tokens come from.
        token_url: String,
        /// Where the user signs in (authorization code).
        auth_url: String,
        /// Where the browser returns to; empty for any free port.
        redirect_url: String,
        /// Client id.
        client_id: String,
        /// Client secret.
        client_secret: String,
        /// Scopes, separated by spaces.
        scope: String,
    },
    /// API key.
    ApiKey {
        /// Header or parameter name.
        name: String,
        /// The key.
        value: String,
        /// Where it goes.
        placement: KeyPlacementView,
    },
    /// HTTP Digest.
    Digest {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// AWS Signature Version 4.
    AwsSigV4 {
        /// Access key id.
        access_key_id: String,
        /// Secret access key.
        secret_access_key: String,
        /// Session token; empty if none.
        session_token: String,
        /// Region.
        region: String,
        /// Service.
        service: String,
    },
}

/// Where an API key goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum KeyPlacementView {
    /// As a header.
    #[default]
    Header,
    /// As a query parameter.
    Query,
}

/// How an OAuth 2 token is obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum OAuth2GrantView {
    /// Client credentials.
    #[default]
    ClientCredentials,
    /// Authorization code with PKCE, signing in in the browser.
    AuthorizationCode,
}

/// Whether the server's TLS certificate is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CertificateCheckView {
    /// As the folder or collection around it.
    #[default]
    Inherit,
    /// Check it.
    On,
    /// Accept any certificate.
    Off,
}

impl CertificateCheckView {
    /// The editor's view of `check`.
    #[must_use]
    pub fn from_check(check: CertificateCheck) -> Self {
        match check {
            CertificateCheck::Inherit => Self::Inherit,
            CertificateCheck::On => Self::On,
            CertificateCheck::Off => Self::Off,
        }
    }

    /// The setting the editor describes.
    #[must_use]
    pub fn to_check(self) -> CertificateCheck {
        match self {
            Self::Inherit => CertificateCheck::Inherit,
            Self::On => CertificateCheck::On,
            Self::Off => CertificateCheck::Off,
        }
    }
}

impl AuthDraft {
    /// The editor's view of `auth`.
    #[must_use]
    pub fn from_auth(auth: &Auth) -> Self {
        match auth {
            Auth::Inherit => Self::Inherit,
            Auth::None => Self::None,
            Auth::Basic { username, password } => Self::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            Auth::Bearer { token } => Self::Bearer {
                token: token.clone(),
            },
            Auth::Digest { username, password } => Self::Digest {
                username: username.clone(),
                password: password.clone(),
            },
            Auth::AwsSigV4(aws) => Self::AwsSigV4 {
                access_key_id: aws.access_key_id.clone(),
                secret_access_key: aws.secret_access_key.clone(),
                session_token: aws.session_token.clone(),
                region: aws.region.clone(),
                service: aws.service.clone(),
            },
            Auth::OAuth2(oauth) => Self::OAuth2 {
                grant: match oauth.grant {
                    OAuth2Grant::ClientCredentials => OAuth2GrantView::ClientCredentials,
                    OAuth2Grant::AuthorizationCode => OAuth2GrantView::AuthorizationCode,
                },
                token_url: oauth.token_url.clone(),
                auth_url: oauth.auth_url.clone(),
                redirect_url: oauth.redirect_url.clone(),
                client_id: oauth.client_id.clone(),
                client_secret: oauth.client_secret.clone(),
                scope: oauth.scope.clone(),
            },
            Auth::ApiKey {
                name,
                value,
                placement,
            } => Self::ApiKey {
                name: name.clone(),
                value: value.clone(),
                placement: match placement {
                    KeyPlacement::Header => KeyPlacementView::Header,
                    KeyPlacement::Query => KeyPlacementView::Query,
                },
            },
        }
    }

    /// The authentication the editor describes.
    #[must_use]
    pub fn to_auth(&self) -> Auth {
        match self {
            Self::Inherit => Auth::Inherit,
            Self::None => Auth::None,
            Self::Basic { username, password } => Auth::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            Self::Bearer { token } => Auth::Bearer {
                token: token.clone(),
            },
            Self::Digest { username, password } => Auth::Digest {
                username: username.clone(),
                password: password.clone(),
            },
            Self::AwsSigV4 {
                access_key_id,
                secret_access_key,
                session_token,
                region,
                service,
            } => Auth::AwsSigV4(AwsSigV4 {
                access_key_id: access_key_id.clone(),
                secret_access_key: secret_access_key.clone(),
                session_token: session_token.clone(),
                region: region.clone(),
                service: service.clone(),
            }),
            Self::OAuth2 {
                grant,
                token_url,
                auth_url,
                redirect_url,
                client_id,
                client_secret,
                scope,
            } => Auth::OAuth2(OAuth2 {
                grant: match grant {
                    OAuth2GrantView::ClientCredentials => OAuth2Grant::ClientCredentials,
                    OAuth2GrantView::AuthorizationCode => OAuth2Grant::AuthorizationCode,
                },
                token_url: token_url.clone(),
                auth_url: auth_url.clone(),
                redirect_url: redirect_url.clone(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                scope: scope.clone(),
            }),
            Self::ApiKey {
                name,
                value,
                placement,
            } => Auth::ApiKey {
                name: name.clone(),
                value: value.clone(),
                placement: match placement {
                    KeyPlacementView::Header => KeyPlacement::Header,
                    KeyPlacementView::Query => KeyPlacement::Query,
                },
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_auth.rs"]
mod tests;
