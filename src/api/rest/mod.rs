pub mod health;
pub mod users;

use crate::state::AppState;
use aide::axum::ApiRouter;

pub fn utility_routes() -> ApiRouter<()> {
    health::routes()
}

pub fn auth_routes(state: AppState) -> ApiRouter {
    users::routes(state)
}
