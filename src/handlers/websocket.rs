use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{ClientWsMessage, ServerWsMessage},
    state::AppState,
};

use axum::extract::{Path, State, WebSocketUpgrade};

use axum::extract::ws::Message;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;

/// Dedicated WebSocket handler for user presence.
///
/// The client connects here once on app open, independent of any chat room.
/// - On connect:    a session row is inserted; the DB generates the session UUID.
///                  An initial snapshot of online friends is pushed to the client.
///                  Friends with an active presence socket are notified.
/// - On heartbeat:  the client sends `{"type":"heartbeat","status":"online"|"idle"}`.
/// - On disconnect: the session row is deleted. If no sessions remain, friends
///                  are notified that the user went offline.
///
/// Route: `GET /ws/presence`
pub async fn presence_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<axum::response::Response, ServiceError> {
    let session_id = state.presence_service.connect(user_id).await?;
    let user = state.user_service.find_by_id(user_id).await?;

    Ok(ws.on_upgrade(move |socket| {
        handle_presence_socket(socket, session_id, user_id, user.username, state)
    }))
}

/// Drives the presence WebSocket: sends an initial snapshot, broadcasts status
/// changes to friends, processes heartbeats, and cleans up on disconnect.
async fn handle_presence_socket(
    socket: axum::extract::ws::WebSocket,
    session_id: Uuid,
    user_id: Uuid,
    username: String,
    state: AppState,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Subscribe to presence notifications for this user.
    let mut notify_rx = {
        let mut subs = state.presence_tx.lock().await;
        let tx = subs
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(64).0);
        tx.subscribe()
    };

    // Send initial snapshot of online friends.
    if let Ok(friends) = state.presence_service.online_friends(user_id).await {
        let msg = ServerWsMessage::OnlineFriends { friends };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = ws_tx.send(Message::Text(json.into())).await;
        }
    }

    // Notify friends that this user came online.
    notify_friends(&state, user_id, &username, "online").await;

    // Writer: forward presence broadcasts to this WebSocket.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = notify_rx.recv().await {
            if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Reader: process incoming heartbeats (rate-limited to once per 5s).
    let presence = state.presence_service.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut last_heartbeat = Instant::now() - std::time::Duration::from_secs(5);
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ClientWsMessage::Heartbeat { status }) =
                        serde_json::from_str::<ClientWsMessage>(&text)
                    {
                        if last_heartbeat.elapsed() >= std::time::Duration::from_secs(5) {
                            last_heartbeat = Instant::now();
                            let _ = presence.heartbeat(session_id, &status).await;
                        }
                    }
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

    // Clean up session.
    let _ = state.presence_service.disconnect(session_id).await;

    // If no remaining sessions, notify friends and remove subscriber entry.
    if !state
        .presence_service
        .is_online(user_id)
        .await
        .unwrap_or(true)
    {
        notify_friends(&state, user_id, &username, "offline").await;
        state.presence_tx.lock().await.remove(&user_id);
    }
}

/// Sends a presence update to all online friends of `user_id`.
async fn notify_friends(state: &AppState, user_id: Uuid, username: &str, status: &str) {
    let friends = match state.friendship_service.list_friends(user_id).await {
        Ok(f) => f,
        Err(_) => return,
    };

    let msg = ServerWsMessage::PresenceUpdate {
        user_id,
        username: username.to_string(),
        status: status.to_string(),
    };
    let json = match serde_json::to_string(&msg) {
        Ok(j) => j,
        Err(_) => return,
    };

    let subs = state.presence_tx.lock().await;
    for friend in &friends {
        if let Some(tx) = subs.get(&friend.user_id) {
            let _ = tx.send(json.clone());
        }
    }
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
