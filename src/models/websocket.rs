use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{dm::DmMessageResponse, presence::OnlineFriendResponse};

/// Incoming JSON message from a WebSocket client.
///
/// Clients must send one of these shapes:
/// - `{"type":"heartbeat","status":"online"}`
/// - `{"type":"heartbeat","status":"idle"}`
/// - `{"type":"message","content":"Hello!"}`
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientWsMessage {
    Heartbeat { status: String },
    Message { content: String },
}

/// Outgoing JSON message sent from the server to WebSocket clients.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerWsMessage {
    /// Initial snapshot of the user's online friends, sent immediately on connect.
    OnlineFriends { friends: Vec<OnlineFriendResponse> },
    /// A friend's presence status changed (online/offline).
    PresenceUpdate {
        user_id: Uuid,
        username: String,
        status: String,
    },
    /// A DM message was persisted for a conversation this user belongs to.
    NewMessage {
        conversation_id: Uuid,
        message: DmMessageResponse,
    },
}
