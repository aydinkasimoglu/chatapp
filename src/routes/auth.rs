use crate::{handlers, state::AppState};

use axum::{Router, routing::post};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::auth::login_handler))
        .route("/signup", post(handlers::auth::signup_handler))
}
