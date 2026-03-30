mod error;
mod extractors;
mod handlers;
mod models;
mod repositories;
mod routes;
mod services;
mod state;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Mutex, time::Instant};

use crate::{
    repositories::{
        blocks::BlockRepository, friendship::FriendshipRepository, presence::PresenceRepository,
        refresh_token::RefreshTokenRepository, server::ServerRepository, user::UserRepository,
    },
    services::{
        auth::AuthService, blocks::BlockService, friendship::FriendshipService,
        presence::PresenceService, server::ServerService, user::UserService,
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

    const PRESENCE_CLEANUP_PERIOD: Duration = Duration::from_secs(15);
    const REFRESH_TOKEN_CLEANUP_PERIOD: Duration = Duration::from_hours(24);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    AuthService::install_crypto_provider();

    let user_repository = UserRepository::new(pool.clone());
    let server_repository = ServerRepository::new(pool.clone());
    let friendship_repository = FriendshipRepository::new(pool.clone());
    let presence_repository = PresenceRepository::new(pool.clone());
    let block_repository = BlockRepository::new(pool.clone());
    let refresh_token_repository = RefreshTokenRepository::new(pool);

    let shared_state = AppState {
        auth_service: AuthService::new(
            user_repository.clone(),
            refresh_token_repository.clone(),
            jwt_secret,
        ),
        user_service: UserService::new(user_repository.clone()),
        server_service: ServerService::new(server_repository),
        friendship_service: FriendshipService::new(
            friendship_repository.clone(),
            user_repository.clone(),
            block_repository.clone(),
        ),
        block_service: BlockService::new(block_repository, friendship_repository, user_repository),
        presence_service: PresenceService::new(presence_repository.clone()),
        rooms: Arc::new(Mutex::new(HashMap::new())),
        presence_tx: Arc::new(Mutex::new(HashMap::new())),
    };

    // Evict stale presence sessions every 15 seconds.
    // This handles clients that crash without sending a clean disconnect.
    let cleanup_presence_service = shared_state.presence_service.clone();
    let cleanup_presence_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval_at(
            Instant::now() + PRESENCE_CLEANUP_PERIOD,
            PRESENCE_CLEANUP_PERIOD,
        );
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = cleanup_presence_service.cleanup_stale().await {
                eprintln!("Presence cleanup error: {:?}", e);
            }
        }
    });

    let cleanup_auth_service = shared_state.auth_service.clone();
    let cleanup_refresh_token_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval_at(
            Instant::now() + REFRESH_TOKEN_CLEANUP_PERIOD,
            REFRESH_TOKEN_CLEANUP_PERIOD,
        );
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            match cleanup_auth_service.delete_expired_refresh_tokens().await {
                Ok(n) if n > 0 => println!("Expired token cleanup: removed {} rows", n),
                Err(e) => eprintln!("Expired token cleanup error: {:?}", e),
                _ => {}
            }
        }
    });

    let app = Router::new()
        .merge(routes::auth::router())
        .nest("/users", routes::users::router())
        .nest("/servers", routes::servers::router())
        .nest("/friends", routes::friends::router())
        .nest("/blocks", routes::blocks::router())
        .nest("/ws", routes::websocket::router())
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    println!("Shutting down background tasks...");
    cleanup_presence_handle.abort();
    cleanup_refresh_token_handle.abort();
    let _ = tokio::join!(cleanup_presence_handle, cleanup_refresh_token_handle);
    println!("Shutdown complete.");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Shutdown signal received");
}
