use crate::{handlers, state::AppState};

use axum::{Router, routing::post};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::auth::login_handler))
        .route("/signup", post(handlers::auth::signup_handler))
        .route("/auth/refresh", post(handlers::auth::refresh_handler))
        .route("/auth/logout", post(handlers::auth::logout_handler))
}
