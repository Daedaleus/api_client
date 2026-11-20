//! View models: plain data for the UI, serialized as camelCase JSON.
//!
//! - [`response`]: results of request runs.
//! - [`body`]: the body preview of a response.
//! - [`collection`]: the request tree and saved requests for editing.

pub mod body;
pub mod collection;
pub mod response;

pub use body::BodyView;
pub use collection::{BodyDraft, CollectionView, HeaderDraft, RequestDraft, TreeItemView};
pub use response::{HeaderView, ResponseView, RunOutcomeView, StatusClassView};
