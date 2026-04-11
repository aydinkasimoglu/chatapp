use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserBlock {
    pub block_id: Uuid,
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Join query result for listing a user's blocked users (includes profile data).
#[derive(Debug, Clone, FromRow)]
pub struct BlockRecord {
    pub block_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// API response returned when a user is blocked.
#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub block_id: Uuid,
    pub blocked_user_id: Uuid,
    pub blocked_username: String,
    pub blocked_email: String,
    pub created_at: DateTime<Utc>,
}

impl From<BlockRecord> for BlockResponse {
    fn from(record: BlockRecord) -> Self {
        Self {
            block_id: record.block_id,
            blocked_user_id: record.user_id,
            blocked_username: record.username,
            blocked_email: record.email,
            created_at: record.created_at,
        }
    }
}
