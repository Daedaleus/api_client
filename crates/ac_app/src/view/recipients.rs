//! Who a collection's secrets are encrypted for.

use serde::Serialize;

/// The recipients of a collection and the user's own public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RecipientsView {
    /// The user's own public key, to pass on to the team.
    pub own: String,
    /// Public keys in `.age-recipients`, in file order.
    pub recipients: Vec<String>,
}
