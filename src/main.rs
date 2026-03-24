mod error;
mod extractors;
mod handlers;
mod models;
mod repositories;
mod services;
mod state;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    repositories::{
        blocks::BlockRepository, friendship::FriendshipRepository, server::ServerRepository,
        user::UserRepository,
    },
    services::{
        auth::AuthService, blocks::BlockService, friendship::FriendshipService,
        server::ServerService, user::UserService,
    },
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
/// - `GET /users`: Get all users
/// - `GET /users/{user_id}`: Get user by ID
/// - `PUT /users/{user_id}`: Update user (username/email)
/// - `PUT /users/{user_id}/password`: Update user password
/// - `DELETE /users/{user_id}`: Deactivate user
/// - `POST /friends/requests`: Send a friend request
/// - `GET /friends`: List accepted friends for the authenticated user
/// - `GET /friends/requests/incoming`: List pending incoming requests
/// - `GET /friends/requests/outgoing`: List pending outgoing requests
/// - `PUT /friends/requests/{friendship_id}/accept`: Accept a friend request
/// - `PUT /friends/requests/{friendship_id}/reject`: Reject a friend request
/// - `DELETE /friends/requests/{friendship_id}/cancel`: Cancel an outgoing pending request
/// - `DELETE /friends/{friendship_id}`: Remove an accepted friendship
/// - `POST /blocks/{target_user_id}`: Block a user
/// - `DELETE /blocks/{target_user_id}`: Unblock a user
/// - `GET /blocks`: List blocked users for the authenticated user
/// - `GET /ws/{room_name}`: WebSocket connection for chat rooms
#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    let user_repository = UserRepository::new(pool.clone());
    let server_repository = ServerRepository::new(pool.clone());
    let friendship_repository = FriendshipRepository::new(pool.clone());
    let block_repository = BlockRepository::new(pool);

    let shared_state = AppState {
        auth_service: AuthService::new(user_repository.clone(), jwt_secret),
        user_service: UserService::new(user_repository.clone()),
        server_service: ServerService::new(server_repository),
        friendship_service: FriendshipService::new(
            friendship_repository.clone(),
            user_repository.clone(),
            block_repository.clone(),
        ),
        block_service: BlockService::new(block_repository, friendship_repository, user_repository),
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
        .route("/friends", get(handlers::friendships::get_friends_handler))
        .route(
            "/friends/requests",
            post(handlers::friendships::send_friend_request_handler),
        )
        .route(
            "/friends/requests/incoming",
            get(handlers::friendships::get_incoming_friend_requests_handler),
        )
        .route(
            "/friends/requests/outgoing",
            get(handlers::friendships::get_outgoing_friend_requests_handler),
        )
        .route(
            "/friends/requests/{friendship_id}/accept",
            put(handlers::friendships::accept_friend_request_handler),
        )
        .route(
            "/friends/requests/{friendship_id}/reject",
            put(handlers::friendships::reject_friend_request_handler),
        )
        .route(
            "/friends/requests/{friendship_id}/cancel",
            delete(handlers::friendships::cancel_friend_request_handler),
        )
        .route(
            "/friends/{friendship_id}",
            delete(handlers::friendships::remove_friend_handler),
        )
        .route("/blocks", get(handlers::blocks::get_blocked_users_handler))
        .route(
            "/blocks/{target_user_id}",
            post(handlers::blocks::block_user_handler)
                .delete(handlers::blocks::unblock_user_handler),
        )
        .route("/servers", post(handlers::servers::create_server_handler))
        .route(
            "/servers/public",
            get(handlers::servers::get_public_servers_handler),
        )
        .route(
            "/servers/mine",
            get(handlers::servers::get_my_servers_handler),
        )
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
