use crate::{handlers, state::AppState};

use axum::{
    Router,
    routing::{get, post},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::servers::create_server_handler))
        .route(
            "/{server_id}",
            get(handlers::servers::get_server_handler)
                .put(handlers::servers::update_server_handler)
                .delete(handlers::servers::delete_server_handler),
        )
        .route(
            "/public",
            get(handlers::servers::get_public_servers_handler),
        )
        .route("/mine", get(handlers::servers::get_my_servers_handler))
}
