use crate::{handlers, state::AppState};

use axum::{
    Router,
    routing::{delete, get, post, put},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::friendships::get_friends_handler))
        .route(
            "/{friendship_id}",
            delete(handlers::friendships::remove_friend_handler),
        )
        .route(
            "/requests",
            post(handlers::friendships::send_friend_request_handler),
        )
        .route(
            "/requests/incoming",
            get(handlers::friendships::get_incoming_friend_requests_handler),
        )
        .route(
            "/requests/outgoing",
            get(handlers::friendships::get_outgoing_friend_requests_handler),
        )
        .route(
            "/requests/{friendship_id}/accept",
            put(handlers::friendships::accept_friend_request_handler),
        )
        .route(
            "/requests/{friendship_id}/reject",
            put(handlers::friendships::reject_friend_request_handler),
        )
        .route(
            "/requests/{friendship_id}/cancel",
            delete(handlers::friendships::cancel_friend_request_handler),
        )
}
