pub mod health;
pub mod users;

use crate::state::AppState;
use aide::axum::ApiRouter;

pub fn utility_routes(state: AppState) -> ApiRouter {
    health::routes(state)
}

pub fn auth_routes(state: AppState) -> ApiRouter {
    users::routes(state)
}
