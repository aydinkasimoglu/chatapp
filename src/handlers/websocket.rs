use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::ClientWsMessage,
    services::presence::PresenceService,
    state::AppState,
};

use axum::extract::{Path, State, WebSocketUpgrade};

use axum::extract::ws::Message;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Dedicated WebSocket handler for user presence.
///
/// The client connects here once on app open, independent of any chat room.
/// - On connect:    a session row is inserted; the DB generates the session UUID.
/// - On heartbeat:  the client sends `{"type":"heartbeat","status":"online"|"idle"}`.
/// - On disconnect: the session row is deleted.
///
/// Route: `GET /ws/presence`
pub async fn presence_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<axum::response::Response, ServiceError> {
    // Insert the session row
    let session_id = state.presence_service.connect(user_id).await?;
    let presence = state.presence_service.clone();

    Ok(ws.on_upgrade(move |socket| handle_presence_socket(socket, session_id, presence)))
}

/// Drives the presence WebSocket: processes heartbeats and cleans up on disconnect.
async fn handle_presence_socket(
    socket: axum::extract::ws::WebSocket,
    session_id: Uuid,
    presence: PresenceService,
) {
    let (mut _tx, mut rx) = socket.split();

    while let Some(Ok(msg)) = rx.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(ClientWsMessage::Heartbeat { status }) =
                    serde_json::from_str::<ClientWsMessage>(&text)
                {
                    let _ = presence.heartbeat(session_id, &status).await;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    let _ = presence.disconnect(session_id).await;
}

/// Handles WebSocket connections for chat rooms.
///
/// Purely responsible for room-based message broadcasting.
/// Presence is tracked separately via the `/ws/presence` endpoint.
///
/// Route: `GET /ws/{room_name}`
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
    Ok(ws.on_upgrade(move |socket| handle_room_socket(socket, tx, rx, user.username)))
}

/// Drives a chat room WebSocket: broadcasts chat messages to all room members.
async fn handle_room_socket(
    socket: axum::extract::ws::WebSocket,
    room_tx: broadcast::Sender<String>,
    mut room_rx: broadcast::Receiver<String>,
    username: String,
) {
    let (mut user_tx, mut user_rx) = socket.split();

    // --- WRITER TASK: forward room broadcasts to this client ---
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = room_rx.recv().await {
            if user_tx.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    // --- READER TASK: receive messages from this client and broadcast ---
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = user_rx.next().await {
            match msg {
                Message::Text(text) => {
                    // Accept both plain text and {"type":"message","content":"..."}
                    let content = serde_json::from_str::<ClientWsMessage>(&text)
                        .ok()
                        .and_then(|m| match m {
                            ClientWsMessage::Message { content } => Some(content),
                            _ => None,
                        })
                        .unwrap_or_else(|| text.to_string());

                    let _ = room_tx.send(format!("{}: {}", username, content));
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
