use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use super::enums::PresenceStatus;

/// DB row for a single active WebSocket session.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserPresence {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub status: PresenceStatus,
    pub last_heartbeat_at: DateTime<Utc>,
    pub connected_at: DateTime<Utc>,
}

/// Query result for the "online friends" aggregation.
/// `status` is 'online' if any session reports online, otherwise 'idle'.
#[derive(Debug, Clone, FromRow)]
pub struct OnlineFriendRecord {
    pub user_id: Uuid,
    pub username: String,
    pub status: PresenceStatus,
    pub last_heartbeat_at: DateTime<Utc>,
}

/// API response shape for an online/idle friend.
#[derive(Debug, Serialize)]
pub struct OnlineFriendResponse {
    pub user_id: Uuid,
    pub username: String,
    pub status: PresenceStatus,
    pub last_heartbeat_at: DateTime<Utc>,
}

impl From<OnlineFriendRecord> for OnlineFriendResponse {
    fn from(record: OnlineFriendRecord) -> Self {
        Self {
            user_id: record.user_id,
            username: record.username,
            status: record.status,
            last_heartbeat_at: record.last_heartbeat_at,
        }
    }
}
