use crate::{handlers, state::AppState};

use axum::{
    Router,
    routing::{get, post},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::blocks::get_blocked_users_handler))
        .route(
            "/{target_user_id}",
            post(handlers::blocks::block_user_handler)
                .delete(handlers::blocks::unblock_user_handler),
        )
}
