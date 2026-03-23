use crate::{error::ServiceError, extractors::AuthenticatedUser, state::AppState};

use axum::extract::{Path, State, WebSocketUpgrade};

use axum::extract::ws::Message;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;

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
) -> Result<axum::response::Response, ServiceError> {
    // Fetch user details
    let user = state.user_service.find_by_id(user_id).await?;

    // Get the room's sender, or create a new one if the room doesn't exist
    let tx = {
        // Lock the mutex to read/write the HashMap safely
        let mut rooms = state.rooms.lock().await;

        if let Some(sender) = rooms.get(&room_name) {
            sender.clone()
        } else {
            let (tx, _) = broadcast::channel(100);
            rooms.insert(room_name.clone(), tx.clone());
            tx
        }
    };

    // Create a receiver for this specific user
    let rx = tx.subscribe();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, tx, rx, user.username)))
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
