pub mod books;
pub mod chapters;
pub mod collections;
pub mod comments;
pub mod health;
pub mod highlights;
pub mod me;
pub mod reviews;
pub mod translations;
pub mod users;

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
        .nest_api_service("/auth", users::routes(state.clone()))
        .merge(books::routes(state.clone()))
        .merge(chapters::routes(state.clone()))
        .merge(reviews::routes(state.clone()))
        .merge(highlights::routes(state.clone()))
        .merge(comments::routes(state.clone()))
        .merge(translations::routes(state.clone()))
        .merge(collections::routes(state.clone()))
        .merge(me::routes(state))
}
