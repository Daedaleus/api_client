//! The sign-in URL of the authorization code flow, secured with PKCE
//! (RFC 7636): a random verifier stays in the app, only its SHA-256 hash
//! goes to the browser, so a code caught on the way is useless to others.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use sha2::{Digest, Sha256};
use url::Url;

use super::OAuthClient;

/// Random bytes of the verifier: 32 give the recommended 43 characters
/// (RFC 7636, section 4.1).
const VERIFIER_BYTES: usize = 32;
/// Random bytes of the `state` that ties the redirect to this sign-in.
const STATE_BYTES: usize = 16;
/// The only challenge method worth using (RFC 7636, section 4.2).
const METHOD_S256: &str = "S256";

/// Query parameters of the sign-in URL (RFC 6749 section 4.1.1, RFC 7636).
mod param {
    pub const RESPONSE_TYPE: &str = "response_type";
    pub const CODE: &str = "code";
    pub const CLIENT_ID: &str = "client_id";
    pub const REDIRECT_URI: &str = "redirect_uri";
    pub const SCOPE: &str = "scope";
    pub const STATE: &str = "state";
    pub const CODE_CHALLENGE: &str = "code_challenge";
    pub const CODE_CHALLENGE_METHOD: &str = "code_challenge_method";
}

/// One sign-in: its secrets and the URL to open.
#[derive(Debug)]
pub(super) struct SignIn {
    /// Sent with the code to the token URL.
    pub verifier: String,
    /// Must come back unchanged with the redirect.
    pub state: String,
    /// Where the browser goes.
    pub url: Url,
}

impl SignIn {
    /// A new sign-in at `auth_url`, returning to `redirect_uri`.
    pub(super) fn new(client: &OAuthClient, auth_url: &Url, redirect_uri: &Url) -> Self {
        let verifier = random_text(VERIFIER_BYTES);
        let state = random_text(STATE_BYTES);
        let mut url = auth_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair(param::RESPONSE_TYPE, param::CODE)
                .append_pair(param::CLIENT_ID, &client.client_id)
                .append_pair(param::REDIRECT_URI, redirect_uri.as_str());
            if !client.scope.is_empty() {
                query.append_pair(param::SCOPE, &client.scope);
            }
            query
                .append_pair(param::STATE, &state)
                .append_pair(param::CODE_CHALLENGE, &challenge(&verifier))
                .append_pair(param::CODE_CHALLENGE_METHOD, METHOD_S256);
        }
        Self {
            verifier,
            state,
            url,
        }
    }
}

/// `BASE64URL(SHA256(verifier))`, the S256 challenge.
pub(super) fn challenge(verifier: &str) -> String {
    BASE64_URL.encode(Sha256::digest(verifier.as_bytes()))
}

/// `bytes` random bytes from the operating system, URL-safe Base64.
fn random_text(bytes: usize) -> String {
    let mut buffer = vec![0; bytes];
    // Without the system's random source no sign-in is safe; it only fails
    // on exotic platforms.
    getrandom::fill(&mut buffer).expect("the operating system's random source works");
    BASE64_URL.encode(buffer)
}

#[cfg(test)]
#[path = "../tests/oauth_pkce.rs"]
mod tests;
