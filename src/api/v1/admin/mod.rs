//! Admin REST exceptions to the GraphQL-first transport policy.
//!
//! Two endpoints live here because they don't fit GraphQL:
//! - `uploads` — multipart-style streaming upload and signed-URL handshake.
//!   GraphQL multipart is awkward; raw bytes + metadata headers is cleaner.
//! - `page_preview` — returns binary PNG/WEBP bytes for the Review pane's
//!   side-by-side PDF preview.
//!
//! Everything else admin-related lives in the GraphQL `AdminQuery` /
//! `AdminMutation` / `SubscriptionRoot` (`src/api/graphql/admin.rs`).

pub mod page_preview;
pub mod uploads;

use crate::state::AppState;
use aide::axum::ApiRouter;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(uploads::routes(state.clone()))
        .merge(page_preview::routes(state))
}
