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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

/// Database row for a direct-message or group conversation.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmConversation {
    pub conversation_id: Uuid,
    pub kind: DmConversationKind,
    pub title: Option<String>,
    pub direct_user_low_id: Option<Uuid>,
    pub direct_user_high_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for a DM conversation membership.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationMember {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub last_read_message_id: Option<Uuid>,
    pub last_read_at: Option<DateTime<Utc>>,
}

/// Joined query result for a DM conversation member with profile data.
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationParticipantRecord {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub joined_at: DateTime<Utc>,
    pub last_read_message_id: Option<Uuid>,
    pub last_read_at: Option<DateTime<Utc>>,
}

/// Query result for listing DM conversations.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationSummaryRecord {
    pub conversation_id: Uuid,
    pub kind: DmConversationKind,
    pub title: Option<String>,
    pub direct_user_low_id: Option<Uuid>,
    pub direct_user_high_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub participant_count: i64,
    pub last_activity_at: DateTime<Utc>,
}

/// Query result for unread DM message counts.
#[derive(Debug, Clone, FromRow)]
pub struct DmUnreadCountRecord {
    pub conversation_id: Uuid,
    pub unread_count: i64,
}

/// Database row for a DM message.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmMessage {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Joined query result for a DM message with sender profile data.
#[derive(Debug, Clone, FromRow)]
pub struct DmMessageRecord {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub content: String,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Database row for a DM message reaction.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmMessageReaction {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub reaction: String,
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct JoinServer {
    pub nickname: Option<String>,
}

#[allow(dead_code)]
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
    pub username: String,
}

/// Payload for creating a DM conversation.
#[derive(Debug, Deserialize)]
pub struct CreateDmConversation {
    pub participant_ids: Vec<Uuid>,
    pub title: Option<String>,
}

/// Payload for sending a DM message.
#[derive(Debug, Deserialize)]
pub struct SendDmMessage {
    pub content: String,
}

/// Payload for updating a DM conversation read cursor.
#[derive(Debug, Deserialize)]
pub struct MarkDmConversationRead {
    pub up_to_message_id: Option<Uuid>,
}

/// Query parameters for listing DM conversations.
#[derive(Debug, Deserialize)]
pub struct DmConversationListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for listing DM messages.
#[derive(Debug, Deserialize)]
pub struct DmMessageListQuery {
    pub limit: Option<i64>,
    pub before_message_id: Option<Uuid>,
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

#[allow(dead_code)]
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

/// API response shape for a DM conversation participant.
#[derive(Debug, Clone, Serialize)]
pub struct DmConversationParticipantResponse {
    pub user_id: Uuid,
    pub username: String,
    pub joined_at: DateTime<Utc>,
    pub last_read_message_id: Option<Uuid>,
    pub last_read_at: Option<DateTime<Utc>>,
}

impl From<DmConversationParticipantRecord> for DmConversationParticipantResponse {
    fn from(record: DmConversationParticipantRecord) -> Self {
        Self {
            user_id: record.user_id,
            username: record.username,
            joined_at: record.joined_at,
            last_read_message_id: record.last_read_message_id,
            last_read_at: record.last_read_at,
        }
    }
}

/// API response shape for a DM message.
#[derive(Debug, Clone, Serialize)]
pub struct DmMessageResponse {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub content: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<DmMessageRecord> for DmMessageResponse {
    fn from(record: DmMessageRecord) -> Self {
        let content = if record.deleted_at.is_some() {
            None
        } else {
            Some(record.content)
        };

        Self {
            message_id: record.message_id,
            conversation_id: record.conversation_id,
            sender_id: record.sender_id,
            sender_username: record.sender_username,
            content,
            edited_at: record.edited_at,
            deleted_at: record.deleted_at,
            created_at: record.created_at,
        }
    }
}

/// API response shape for a DM conversation summary.
#[derive(Debug, Clone, Serialize)]
pub struct DmConversationSummaryResponse {
    pub conversation_id: Uuid,
    pub kind: DmConversationKind,
    pub title: Option<String>,
    pub display_title: String,
    pub direct_partner_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub participant_count: i64,
    pub unread_count: i64,
    pub participants: Vec<DmConversationParticipantResponse>,
    pub last_message: Option<DmMessageResponse>,
}

/// API response shape for a DM conversation detail.
#[derive(Debug, Clone, Serialize)]
pub struct DmConversationResponse {
    pub conversation_id: Uuid,
    pub kind: DmConversationKind,
    pub title: Option<String>,
    pub display_title: String,
    pub direct_partner_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub participant_count: i64,
    pub unread_count: i64,
    pub participants: Vec<DmConversationParticipantResponse>,
    pub last_message: Option<DmMessageResponse>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub token_id: Uuid,
    pub user_id:  Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    /// Short-lived JWT (15 minutes).
    pub access_token: String,
    /// Opaque token used to obtain a new access token.
    pub refresh_token: String,
}

/// Cursor-based API response for paginated DM message lists.
#[derive(Debug, Serialize)]
pub struct CursorPaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub limit: i64,
    pub before_message_id: Option<Uuid>,
    pub next_before_message_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Admin,
    Moderator,
    Member,
}

/// Distinguishes 1:1 direct conversations from group DMs.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "dm_conversation_kind", rename_all = "lowercase")]
pub enum DmConversationKind {
    Direct,
    Group,
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
