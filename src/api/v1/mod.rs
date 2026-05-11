//! REST surface — limited to operations that can't be GraphQL:
//! - `admin/uploads` (multipart file upload)
//! - `admin/ingestion-jobs/{id}/pages/{n}.png` (binary page preview)
//! - `health` (probe endpoint)
//!
//! Everything else lives at `/api/graphql`. See INTEGRATION_PLAN.md §4.

pub mod admin;
pub mod health;

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema, Clone, Copy)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Pagination {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

use crate::state::AppState;
use aide::axum::ApiRouter;

pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(health::routes(state.clone()))
        .merge(admin::routes(state))
}
