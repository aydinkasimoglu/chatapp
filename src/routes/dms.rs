use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::{handlers, state::AppState};

/// Builds the DM HTTP router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/conversations",
            post(handlers::dms::create_conversation_handler)
                .get(handlers::dms::list_conversations_handler),
        )
        .route(
            "/conversations/{conversation_id}",
            get(handlers::dms::get_conversation_handler),
        )
        .route(
            "/conversations/{conversation_id}/messages",
            post(handlers::dms::send_message_handler)
                .get(handlers::dms::list_messages_handler),
        )
        .route(
            "/conversations/{conversation_id}/read",
            patch(handlers::dms::mark_as_read_handler),
        )
        .route("/messages/{message_id}", delete(handlers::dms::delete_message_handler))
}