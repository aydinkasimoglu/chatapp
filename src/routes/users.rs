use crate::{handlers, state::AppState};

use axum::{
    Router,
    routing::{get, put},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::users::get_users_handler))
        .route(
            "/{user_id}",
            get(handlers::users::get_user_by_id_handler)
                .put(handlers::users::update_user_handler)
                .delete(handlers::users::deactivate_user_handler),
        )
        .route(
            "/{user_id}/password",
            put(handlers::users::update_password_handler),
        )
}
