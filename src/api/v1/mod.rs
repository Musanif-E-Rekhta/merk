pub mod health;
pub mod users;

use crate::state::AppState;
use aide::axum::ApiRouter;

pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(health::routes(state.clone()))
        .nest_api_service("/auth", users::routes(state))
}
