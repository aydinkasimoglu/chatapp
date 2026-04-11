use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::enums::FriendshipStatus;

#[derive(Debug, Clone, FromRow)]
pub struct Friendship {
    pub friendship_id: Uuid,
    pub requester_id: Uuid,
    pub addressee_id: Uuid,
    pub status: FriendshipStatus,
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FriendRecord {
    pub friendship_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub friends_since: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PendingFriendRequestRecord {
    pub friendship_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
}

/// Payload for sending a friend request.
#[derive(Debug, Deserialize)]
pub struct FriendRequestPayload {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct FriendshipResponse {
    pub friendship_id: Uuid,
    pub requester_id: Uuid,
    pub addressee_id: Uuid,
    pub status: FriendshipStatus,
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Friendship> for FriendshipResponse {
    fn from(friendship: Friendship) -> Self {
        Self {
            friendship_id: friendship.friendship_id,
            requester_id: friendship.requester_id,
            addressee_id: friendship.addressee_id,
            status: friendship.status,
            responded_at: friendship.responded_at,
            created_at: friendship.created_at,
            updated_at: friendship.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FriendResponse {
    pub friendship_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub friends_since: DateTime<Utc>,
}

impl From<FriendRecord> for FriendResponse {
    fn from(record: FriendRecord) -> Self {
        Self {
            friendship_id: record.friendship_id,
            user_id: record.user_id,
            username: record.username,
            email: record.email,
            friends_since: record.friends_since,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PendingFriendRequestResponse {
    pub friendship_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
}

impl From<PendingFriendRequestRecord> for PendingFriendRequestResponse {
    fn from(record: PendingFriendRequestRecord) -> Self {
        Self {
            friendship_id: record.friendship_id,
            user_id: record.user_id,
            username: record.username,
            email: record.email,
            status: record.status,
            created_at: record.created_at,
        }
    }
}
