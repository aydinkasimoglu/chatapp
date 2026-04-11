use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::enums::DmConversationKind;

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
