//! The scripts of a request as the editor shows and returns them.

use ac_domain::script::Scripts;
use serde::{Deserialize, Serialize};

/// Scripts before sending and after the response; empty = none.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ScriptsDraft {
    /// Runs before the request is sent.
    #[serde(default)]
    pub pre_request: String,
    /// Runs after the response arrived.
    #[serde(default)]
    pub post_response: String,
}

impl ScriptsDraft {
    /// The editor's view of `scripts`.
    #[must_use]
    pub fn from_scripts(scripts: &Scripts) -> Self {
        Self {
            pre_request: scripts.pre_request.clone(),
            post_response: scripts.post_response.clone(),
        }
    }

    /// The scripts the editor describes.
    #[must_use]
    pub fn to_scripts(&self) -> Scripts {
        Scripts {
            pre_request: self.pre_request.clone(),
            post_response: self.post_response.clone(),
        }
    }
}
