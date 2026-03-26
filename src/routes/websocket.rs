use crate::{handlers, state::AppState};

use axum::{Router, routing::get};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/presence", get(handlers::websocket::presence_handler))
        .route("/{room_name}", get(handlers::websocket::room_handler))
}
