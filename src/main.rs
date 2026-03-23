mod error;
mod extractors;
mod handlers;
mod models;
mod repositories;
mod services;
mod state;

use axum::{
    Router,
    routing::{get, post, put},
};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    repositories::{server::ServerRepository, user::UserRepository},
    services::{auth::AuthService, server::ServerService, user::UserService},
    state::AppState,
};

/// Entry point for the chat application server.
///
/// Initializes the database connection, sets up services, configures routes,
/// and starts an HTTP server listening on localhost:3000.
///
/// # Environment Variables
/// - `DATABASE_URL`: PostgreSQL connection string (required)
/// - `JWT_SECRET`: Secret key for JWT token signing (required)
///
/// # Routes
/// - `POST /login`: User login endpoint
/// - `POST /signup`: User registration endpoint
/// - `POST /users`: Create a new user
/// - `GET /users`: Get all users
/// - `GET /users/{user_id}`: Get user by ID
/// - `PUT /users/{user_id}`: Update user (username/email)
/// - `PUT /users/{user_id}/password`: Update user password
/// - `DELETE /users/{user_id}`: Deactivate user
/// - `GET /ws/{room_name}`: WebSocket connection for chat rooms
#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    // Connect to Database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    let user_repository = UserRepository::new(pool.clone());

    // Initialize State
    let shared_state = AppState {
        auth_service: AuthService::new(user_repository.clone(), jwt_secret),
        user_service: UserService::new(user_repository),
        server_service: ServerService::new(ServerRepository::new(pool)),
        rooms: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/ws/{room_name}", get(handlers::websocket::room_handler))
        .route("/users", get(handlers::users::get_users_handler))
        .route(
            "/users/{user_id}",
            get(handlers::users::get_user_by_id_handler)
                .put(handlers::users::update_user_handler)
                .delete(handlers::users::deactivate_user_handler),
        )
        .route(
            "/users/{user_id}/password",
            put(handlers::users::update_password_handler),
        )
        .route("/login", post(handlers::auth::login_handler))
        .route("/signup", post(handlers::auth::signup_handler))
        .route("/servers", post(handlers::servers::create_server_handler))
        .route("/servers/public", get(handlers::servers::get_public_servers_handler))
        .route("/servers/mine", get(handlers::servers::get_my_servers_handler))
        .route(
            "/servers/{server_id}",
            get(handlers::servers::get_server_handler)
                .put(handlers::servers::update_server_handler)
                .delete(handlers::servers::delete_server_handler),
        )
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
