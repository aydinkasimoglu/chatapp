use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::models::{
    DmConversation, DmConversationKind, DmConversationParticipantRecord,
    DmConversationSummaryRecord, DmMessage, DmMessageRecord, DmUnreadCountRecord,
};

/// Access flags for a DM conversation request.
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationAccess {
    pub conversation_exists: bool,
    pub is_member: bool,
}

/// Data access object for DM conversations and messages.
#[derive(Clone)]
pub struct DmRepository {
    pool: PgPool,
}

impl DmRepository {
    /// Creates a new `DmRepository` instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a DM conversation and its membership rows in a single transaction.
    pub async fn create_conversation(
        &self,
        creator_id: Uuid,
        participant_ids: &[Uuid],
        kind: DmConversationKind,
        title: Option<&str>,
    ) -> Result<DmConversation, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (direct_user_low_id, direct_user_high_id) = if kind == DmConversationKind::Direct {
            match participant_ids {
                [direct_user_low_id, direct_user_high_id] => {
                    (Some(*direct_user_low_id), Some(*direct_user_high_id))
                }
                _ => {
                    return Err(sqlx::Error::Protocol(
                        "direct conversations require exactly two participants".into(),
                    ));
                }
            }
        } else {
            (None, None)
        };

        let conversation = sqlx::query_as::<_, DmConversation>(
            r#"
            INSERT INTO dm_conversations (
                kind,
                title,
                direct_user_low_id,
                direct_user_high_id,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                conversation_id,
                kind,
                title,
                direct_user_low_id,
                direct_user_high_id,
                created_by,
                created_at,
                updated_at
            "#,
        )
        .bind(kind)
        .bind(title)
        .bind(direct_user_low_id)
        .bind(direct_user_high_id)
        .bind(creator_id)
        .fetch_one(&mut *tx)
        .await?;

        for participant_id in participant_ids {
            sqlx::query(
                r#"
                INSERT INTO dm_conversation_members (conversation_id, user_id)
                VALUES ($1, $2)
                "#,
            )
            .bind(conversation.conversation_id)
            .bind(*participant_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(conversation)
    }

    /// Returns the access state for a conversation request.
    pub async fn get_conversation_access(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<DmConversationAccess, sqlx::Error> {
        sqlx::query_as::<_, DmConversationAccess>(
            r#"
            SELECT
                EXISTS (
                    SELECT 1
                    FROM dm_conversations
                    WHERE conversation_id = $1
                ) AS conversation_exists,
                EXISTS (
                    SELECT 1
                    FROM dm_conversation_members
                    WHERE conversation_id = $1 AND user_id = $2
                ) AS is_member
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Returns a conversation by identifier.
    pub async fn find_conversation_by_id(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<DmConversation>, sqlx::Error> {
        sqlx::query_as::<_, DmConversation>(
            r#"
            SELECT
                conversation_id,
                kind,
                title,
                direct_user_low_id,
                direct_user_high_id,
                created_by,
                created_at,
                updated_at
            FROM dm_conversations
            WHERE conversation_id = $1
            "#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Returns a direct conversation for the canonical user pair, if one exists.
    pub async fn find_direct_conversation_by_pair(
        &self,
        direct_user_low_id: Uuid,
        direct_user_high_id: Uuid,
    ) -> Result<Option<DmConversation>, sqlx::Error> {
        sqlx::query_as::<_, DmConversation>(
            r#"
            SELECT
                conversation_id,
                kind,
                title,
                direct_user_low_id,
                direct_user_high_id,
                created_by,
                created_at,
                updated_at
            FROM dm_conversations
            WHERE kind = 'direct'
              AND direct_user_low_id = $1
              AND direct_user_high_id = $2
            "#,
        )
        .bind(direct_user_low_id)
        .bind(direct_user_high_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Lists conversations visible to the given user.
    pub async fn list_conversations(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DmConversationSummaryRecord>, sqlx::Error> {
        sqlx::query_as::<_, DmConversationSummaryRecord>(
            r#"
            WITH member_conversations AS (
                SELECT conversation_id
                FROM dm_conversation_members
                WHERE user_id = $1
            )
            SELECT
                dc.conversation_id,
                dc.kind,
                dc.title,
                dc.direct_user_low_id,
                dc.direct_user_high_id,
                dc.created_by,
                dc.created_at,
                dc.updated_at,
                COUNT(DISTINCT dcm_all.user_id)::BIGINT AS participant_count,
                COALESCE(MAX(dm.created_at), dc.created_at) AS last_activity_at
            FROM member_conversations mc
            JOIN dm_conversations dc
              ON dc.conversation_id = mc.conversation_id
            JOIN dm_conversation_members dcm_all
              ON dcm_all.conversation_id = dc.conversation_id
            LEFT JOIN dm_messages dm
              ON dm.conversation_id = dc.conversation_id
            GROUP BY
                dc.conversation_id,
                dc.kind,
                dc.title,
                dc.direct_user_low_id,
                dc.direct_user_high_id,
                dc.created_by,
                dc.created_at,
                dc.updated_at
            ORDER BY last_activity_at DESC, dc.conversation_id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Lists participant rows for one or more conversations.
    pub async fn list_participants_for_conversations(
        &self,
        conversation_ids: &[Uuid],
    ) -> Result<Vec<DmConversationParticipantRecord>, sqlx::Error> {
        if conversation_ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, DmConversationParticipantRecord>(
            r#"
            SELECT
                dcm.conversation_id,
                dcm.user_id,
                u.username,
                dcm.joined_at,
                dcm.last_read_message_id,
                dcm.last_read_at
            FROM dm_conversation_members dcm
            JOIN users u ON u.user_id = dcm.user_id
            WHERE dcm.conversation_id = ANY($1)
            ORDER BY dcm.conversation_id ASC, dcm.joined_at ASC, u.username ASC
            "#,
        )
        .bind(conversation_ids.to_vec())
        .fetch_all(&self.pool)
        .await
    }

    /// Lists participant user identifiers for a conversation.
    pub async fn list_participant_user_ids(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT user_id
            FROM dm_conversation_members
            WHERE conversation_id = $1
            ORDER BY joined_at ASC, user_id ASC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Lists the latest message for each requested conversation.
    pub async fn list_latest_messages_for_conversations(
        &self,
        conversation_ids: &[Uuid],
    ) -> Result<Vec<DmMessageRecord>, sqlx::Error> {
        if conversation_ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, DmMessageRecord>(
            r#"
            SELECT DISTINCT ON (dm.conversation_id)
                dm.message_id,
                dm.conversation_id,
                dm.sender_id,
                u.username AS sender_username,
                dm.content,
                dm.edited_at,
                dm.deleted_at,
                dm.created_at
            FROM dm_messages dm
            JOIN users u ON u.user_id = dm.sender_id
            WHERE dm.conversation_id = ANY($1)
            ORDER BY dm.conversation_id ASC, dm.message_id DESC
            "#,
        )
        .bind(conversation_ids.to_vec())
        .fetch_all(&self.pool)
        .await
    }

    /// Lists unread message counts for the given user and conversations.
    pub async fn list_unread_counts(
        &self,
        user_id: Uuid,
        conversation_ids: &[Uuid],
    ) -> Result<Vec<DmUnreadCountRecord>, sqlx::Error> {
        if conversation_ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, DmUnreadCountRecord>(
            r#"
            SELECT
                dcm.conversation_id,
                COALESCE(COUNT(dm.message_id), 0)::BIGINT AS unread_count
            FROM dm_conversation_members dcm
            LEFT JOIN dm_messages dm
              ON dm.conversation_id = dcm.conversation_id
             AND dm.deleted_at IS NULL
             AND dm.sender_id <> $1
             AND (
                    dcm.last_read_message_id IS NULL
                    OR dm.message_id > dcm.last_read_message_id
                 )
            WHERE dcm.user_id = $1
              AND dcm.conversation_id = ANY($2)
            GROUP BY dcm.conversation_id
            ORDER BY dcm.conversation_id ASC
            "#,
        )
        .bind(user_id)
        .bind(conversation_ids.to_vec())
        .fetch_all(&self.pool)
        .await
    }

    /// Inserts a new message into a conversation.
    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<DmMessage, sqlx::Error> {
        sqlx::query_as::<_, DmMessage>(
            r#"
            INSERT INTO dm_messages (conversation_id, sender_id, content)
            VALUES ($1, $2, $3)
            RETURNING
                message_id,
                conversation_id,
                sender_id,
                content,
                edited_at,
                deleted_at,
                created_at
            "#,
        )
        .bind(conversation_id)
        .bind(sender_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await
    }

    /// Lists messages for a conversation using a descending message-id cursor.
    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        before_message_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<DmMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, DmMessageRecord>(
            r#"
            SELECT
                dm.message_id,
                dm.conversation_id,
                dm.sender_id,
                u.username AS sender_username,
                dm.content,
                dm.edited_at,
                dm.deleted_at,
                dm.created_at
            FROM dm_messages dm
            JOIN users u ON u.user_id = dm.sender_id
            WHERE dm.conversation_id = $1
              AND ($2::uuid IS NULL OR dm.message_id < $2)
            ORDER BY dm.message_id DESC
            LIMIT $3
            "#,
        )
        .bind(conversation_id)
        .bind(before_message_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Finds a message row by identifier.
    pub async fn find_message_by_id(
        &self,
        message_id: Uuid,
    ) -> Result<Option<DmMessage>, sqlx::Error> {
        sqlx::query_as::<_, DmMessage>(
            r#"
            SELECT
                message_id,
                conversation_id,
                sender_id,
                content,
                edited_at,
                deleted_at,
                created_at
            FROM dm_messages
            WHERE message_id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Finds a message row with sender profile data by identifier.
    pub async fn find_message_record(
        &self,
        message_id: Uuid,
    ) -> Result<Option<DmMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, DmMessageRecord>(
            r#"
            SELECT
                dm.message_id,
                dm.conversation_id,
                dm.sender_id,
                u.username AS sender_username,
                dm.content,
                dm.edited_at,
                dm.deleted_at,
                dm.created_at
            FROM dm_messages dm
            JOIN users u ON u.user_id = dm.sender_id
            WHERE dm.message_id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Returns whether a message belongs to a specific conversation.
    pub async fn message_belongs_to_conversation(
        &self,
        message_id: Uuid,
        conversation_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM dm_messages
                WHERE message_id = $1 AND conversation_id = $2
            )
            "#,
        )
        .bind(message_id)
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Returns the configured maximum group DM member count, if set.
    pub async fn current_group_member_limit(&self) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<i32>>(
            r#"
            SELECT NULLIF(current_setting('app.dm_group_max_members', true), '')::INT
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Updates a member's read cursor.
    pub async fn mark_as_read(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        up_to_message_id: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE dm_conversation_members
            SET
                last_read_message_id = CASE
                    WHEN $3::uuid IS NULL THEN last_read_message_id
                    WHEN last_read_message_id IS NULL OR last_read_message_id < $3 THEN $3
                    ELSE last_read_message_id
                END,
                last_read_at = NOW()
            WHERE conversation_id = $1
              AND user_id = $2
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(up_to_message_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Soft-deletes a DM message.
    pub async fn soft_delete_message(&self, message_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE dm_messages
            SET deleted_at = NOW()
            WHERE message_id = $1
              AND deleted_at IS NULL
            "#,
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}