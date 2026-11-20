//! View models: plain data for the UI, serialized as camelCase JSON.
//!
//! - [`response`]: results of request runs.
//! - [`body`]: the body preview of a response.
//! - [`collection`]: the request tree and saved requests for editing.
//! - [`environment`]: environments and their variables.
//! - [`recipients`]: who secrets are encrypted for.
//! - [`variable`]: variable rows and where variables come from when sending.

pub mod body;
pub mod collection;
pub mod environment;
pub mod recipients;
pub mod response;
pub mod variable;

pub use body::BodyView;
pub use collection::{BodyDraft, CollectionView, HeaderDraft, RequestDraft, TreeItemView};
pub use environment::EnvironmentDraft;
pub use recipients::RecipientsView;
pub use response::{HeaderView, ResponseView, RunOutcomeView, StatusClassView};
pub use variable::{SendContext, VariableDraft};
