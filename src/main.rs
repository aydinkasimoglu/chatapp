use axum::{Router, middleware, extract::Request, response::Response};
use chatapp::{routes, state::AppState};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, time::Instant};
use tracing::{error, info, warn};

/// Entry point for the chat application server.
///
/// Initializes the database connection, sets up services, configures routes,
/// and starts an HTTP server.
///
/// # Environment Variables
/// - `DATABASE_URL`: PostgreSQL connection string (required)
/// - `JWT_SECRET`: Secret key for JWT token signing (required)
/// - `HOST`: Bind address (default: `0.0.0.0`)
/// - `PORT`: Listen port (default: `3000`)
/// - `DB_MAX_CONNECTIONS`: Maximum database pool connections (default: `20`)
/// - `RUST_LOG`: Tracing filter directive (default: `info`)
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(error) = run().await {
        error!(error = ?error, "server startup failed");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_url = required_env("DATABASE_URL")?;
    let jwt_secret = required_env("JWT_SECRET")?;
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = parse_env_or("PORT", 3000);
    let db_max_connections: u32 = parse_env_or("DB_MAX_CONNECTIONS", 20);

    const PRESENCE_CLEANUP_PERIOD: Duration = Duration::from_secs(15);
    const REFRESH_TOKEN_CLEANUP_PERIOD: Duration = Duration::from_hours(24);

    let pool = PgPoolOptions::new()
        .max_connections(db_max_connections)
        .connect(&db_url)
        .await?;

    let shared_state = AppState::new(pool, jwt_secret)?;

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
                error!("Presence cleanup error: {:?}", e);
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
                Ok(n) if n > 0 => info!("Expired token cleanup: removed {} rows", n),
                Err(e) => error!("Expired token cleanup error: {:?}", e),
                _ => {}
            }
        }
    });

    let app = Router::new()
        .merge(routes::auth::router())
        .merge(routes::dms::router())
        .nest("/users", routes::users::router())
        .nest("/servers", routes::servers::router())
        .nest("/friends", routes::friends::router())
        .nest("/blocks", routes::blocks::router())
        .nest("/ws", routes::websocket::router())
        .layer(middleware::from_fn(log_request))
        .with_state(shared_state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Server listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down background tasks...");
    cleanup_presence_handle.abort();
    cleanup_refresh_token_handle.abort();
    let _ = tokio::join!(cleanup_presence_handle, cleanup_refresh_token_handle);
    info!("Shutdown complete.");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            error!(error = ?error, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                error!(error = ?error, "failed to install SIGTERM handler");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

fn required_env(key: &str) -> Result<String, std::io::Error> {
    std::env::var(key).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{key} must be set"))
    })
}

async fn log_request(req: Request, next: middleware::Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();
    let response = next.run(req).await;
    info!("{} {} {} {:.2?}", method, response.status(), uri, start.elapsed());
    response
}

/// Parse an environment variable into `T`, warning and falling back to `default`
/// if the variable is set but contains an invalid value.
fn parse_env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    match std::env::var(key) {
        Ok(val) => match val.parse() {
            Ok(parsed) => parsed,
            Err(_) => {
                warn!("{key}={val:?} is not valid, falling back to default");
                default
            }
        },
        Err(_) => default,
    }
}
