use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

// =============================================================
// CORE DATABASE MODELS
// =============================================================

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Server {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ServerMember {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub nickname: Option<String>,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

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
pub struct UserBlock {
    pub block_id:   Uuid,
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Join query result for listing a user's blocked users (includes profile data).
#[derive(Debug, Clone, FromRow)]
pub struct BlockRecord {
    pub block_id:  Uuid,
    pub user_id:   Uuid,
    pub username:  String,
    pub email:     String,
    pub created_at: DateTime<Utc>,
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

// =============================================================
// REQUEST MODELS (incoming API payloads)
// =============================================================

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePassword {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateServer {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServer {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct JoinServer {
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMember {
    pub nickname: Option<String>,
    pub role: Option<MemberRole>,
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
/// Payload for sending a friend request.
pub struct FriendRequestPayload {
    pub addressee_id: Uuid,
}

// =============================================================
// RESPONSE MODELS (outgoing API shapes)
// =============================================================

/// API response returned when a user is blocked.
#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub block_id:          Uuid,
    pub blocked_user_id:   Uuid,
    pub blocked_username:  String,
    pub blocked_email:     String,
    pub created_at:        DateTime<Utc>,
}

impl From<BlockRecord> for BlockResponse {
    fn from(record: BlockRecord) -> Self {
        Self {
            block_id:         record.block_id,
            blocked_user_id:  record.user_id,
            blocked_username: record.username,
            blocked_email:    record.email,
            created_at:       record.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            user_id: user.user_id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerResponse {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Server> for ServerResponse {
    fn from(server: Server) -> Self {
        Self {
            server_id: server.server_id,
            owner_id: server.owner_id,
            name: server.name,
            description: server.description,
            is_public: server.is_public,
            created_at: server.created_at,
            updated_at: server.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerMemberResponse {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub nickname: Option<String>,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

impl From<ServerMember> for ServerMemberResponse {
    fn from(member: ServerMember) -> Self {
        Self {
            user_id: member.user_id,
            server_id: member.server_id,
            nickname: member.nickname,
            role: member.role,
            joined_at: member.joined_at,
        }
    }
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

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

// =============================================================
// ENUM
// =============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "friendship_status", rename_all = "lowercase")]
pub enum FriendshipStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Admin,
    Moderator,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "presence_status", rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Idle,
}

// =============================================================
// PRESENCE
// =============================================================

/// DB row for a single active WebSocket session.
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
    fn from(r: OnlineFriendRecord) -> Self {
        Self {
            user_id: r.user_id,
            username: r.username,
            status: r.status,
            last_heartbeat_at: r.last_heartbeat_at,
        }
    }
}

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
