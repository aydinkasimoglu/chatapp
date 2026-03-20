use futures::{sink::SinkExt, stream::StreamExt};

use axum::{
    Json,
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::StatusCode,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{AuthRequest, AuthResponse, CreateUser, UpdatePassword, UpdateUser, UserResponse},
    state::AppState,
};

/// Authenticates a user and returns a JWT token.
///
/// Accepts email and password credentials, verifies them against the database,
/// and returns a JWT token for authenticated requests.
///
/// # Arguments
/// * `state` - Application state containing auth service
/// * `payload` - Authentication request with email and password
///
/// # Returns
/// A JWT token wrapped in an `AuthResponse` on success
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ServiceError> {
    let response = state.auth_service.authenticate(payload).await?;
    Ok(Json(response))
}

/// Creates a new user account.
///
/// Registers a new user with the provided username and email.
/// Returns HTTP 201 Created on success.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `payload` - User creation data (username, email)
///
/// # Returns
/// HTTP 201 with the created user on success
pub async fn signup_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserResponse>), ServiceError> {
    let user = state.user_service.create_user(payload).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// Updates an existing user's information.
///
/// Updates the username and/or email for the specified user.
/// Only provided fields are updated (partial updates supported).
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to update
/// * `payload` - Update data with optional username and email
///
/// # Returns
/// The updated user on success
pub async fn update_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, ServiceError> {
    let user = state.user_service.update_user(user_id, payload).await?;
    Ok(Json(user.into()))
}

/// Deactivates a user by UUID.
///
/// Removes the specified user from the database.
/// Returns HTTP 204 No Content on success.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to delete
///
/// # Returns
/// HTTP 204 No Content on success
pub async fn deactivate_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state.user_service.deactivate_user(user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Changes a user's password.
///
/// Requires the current password for verification. Returns 204 No Content
/// on success.
pub async fn update_password_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdatePassword>,
) -> Result<StatusCode, ServiceError> {
    state
        .user_service
        .change_password(user_id, payload.current_password, payload.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Retrieves all users from the database.
///
/// Fetches a list of all registered users.
///
/// # Arguments
/// * `state` - Application state containing user service
///
/// # Returns
/// A vector of all users
pub async fn get_users_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, ServiceError> {
    let users = state.user_service.get_all_users().await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

/// Retrieves a specific user by ID.
///
/// Fetches user information for the given UUID.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to retrieve
///
/// # Returns
/// The requested user on success, or NotFound error if user doesn't exist
pub async fn get_user_by_id_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserResponse>, ServiceError> {
    let user = state.user_service.get_user_by_id(user_id).await?;
    Ok(Json(user.into()))
}

/// Handles WebSocket connections for chat rooms.
///
/// Establishes a WebSocket connection for a specific chat room, enabling
/// real-time message broadcasting between all connected clients in that room.
/// Creates a new room if it doesn't exist.
///
/// # Arguments
/// * `ws` - WebSocket upgrade request
/// * `room_name` - Name of the chat room to connect to
/// * `state` - Application state containing rooms HashMap
///
/// # Returns
/// WebSocket response for the upgrade
pub async fn room_handler(
    ws: WebSocketUpgrade,
    Path(room_name): Path<String>,
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> axum::response::Response {
    // Fetch user details
    let user = state.user_service.get_user_by_id(user_id).await.unwrap();

    // Lock the mutex to read/write the HashMap safely
    let mut rooms = state.rooms.lock().await;

    // Get the room's sender, or create a new one if the room doesn't exist
    let tx = if let Some(sender) = rooms.get(&room_name) {
        sender.clone()
    } else {
        // Create a new channel that can hold 100 messages
        let (tx, _) = broadcast::channel(100);
        rooms.insert(room_name.clone(), tx.clone());
        tx
    };

    // Create a receiver for this specific user
    let rx = tx.subscribe();

    // Unlock the mutex so other users can connect
    drop(rooms);

    ws.on_upgrade(move |socket| handle_socket(socket, tx, rx, user.username))
}

/// Handles individual WebSocket socket connections within a room.
///
/// Manages bidirectional communication for a single client:
/// - Receives messages from the client and broadcasts them to the room
/// - Receives broadcasted room messages and sends them to the client
/// Runs message sender and receiver tasks concurrently using tokio::select.
///
/// # Arguments
/// * `socket` - The WebSocket connection
/// * `room_tx` - Broadcast sender for the room
/// * `room_rx` - Broadcast receiver for the room
async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    room_tx: broadcast::Sender<String>,
    mut room_rx: broadcast::Receiver<String>,
    username: String,
) {
    // Split the socket into sender and receiver
    let (mut user_tx, mut user_rx) = socket.split();

    // --- WRITER TASK ---
    let mut send_task = tokio::spawn(async move {
        // room_rx.recv() gets messages that ANYONE sent to this room
        while let Ok(message) = room_rx.recv().await {
            // Send the broadcasted message down to this specific user's WebSocket
            if user_tx.send(Message::Text(message.into())).await.is_err() {
                // If sending fails (e.g., client disconnected), break the loop
                break;
            }
        }
    });

    // --- READER TASK ---
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = user_rx.next().await {
            if let Message::Text(text) = msg {
                let message = format!("{}: {}", username, text);
                let _ = room_tx.send(message);
            } else if let Message::Close(_) = msg {
                println!("Client initiated close.");
                break;
            }
        }
    });

    // --- TEARDOWN ---
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("Client disconnected and tasks cleaned up.");
}
