use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{Mutex, broadcast};
use tracing::warn;
use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::{
        CreateDmConversation, DmConversationKind, DmConversationParticipantRecord,
        DmConversationParticipantResponse, DmConversationResponse, DmConversationSummaryResponse,
        DmMessageResponse, ServerWsMessage,
    },
    repositories::{blocks::BlockRepository, dm::DmRepository, user::UserRepository},
};

/// Service for DM conversations and DM messages.
#[derive(Clone)]
pub struct DmService {
    repository: DmRepository,
    user_repository: UserRepository,
    block_repository: BlockRepository,
    presence_tx: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}

impl DmService {
    /// Creates a new `DmService` instance.
    pub fn new(
        repository: DmRepository,
        user_repository: UserRepository,
        block_repository: BlockRepository,
        presence_tx: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
    ) -> Self {
        Self {
            repository,
            user_repository,
            block_repository,
            presence_tx,
        }
    }

    /// Creates a direct conversation or group DM for the authenticated user.
    pub async fn create_conversation(
        &self,
        creator_id: Uuid,
        payload: CreateDmConversation,
    ) -> Result<(DmConversationResponse, bool), ServiceError> {
        let participant_ids = self.normalize_participant_ids(creator_id, &payload.participant_ids);
        let normalized_title = Self::normalize_title(payload.title);

        if participant_ids.len() < 2 {
            return Err(ServiceError::ValidationError(
                "A DM conversation requires at least two distinct participants".to_string(),
            ));
        }

        self.ensure_users_exist(&participant_ids).await?;
        self.ensure_no_blocked_pairs(&participant_ids).await?;

        let kind = if participant_ids.len() == 2 {
            if normalized_title.is_some() {
                return Err(ServiceError::ValidationError(
                    "Direct conversations cannot have a title".to_string(),
                ));
            }

            let existing = self
                .repository
                .find_direct_conversation_by_pair(participant_ids[0], participant_ids[1])
                .await?;

            if let Some(existing) = existing {
                let conversation = self
                    .get_conversation(existing.conversation_id, creator_id)
                    .await?;
                return Ok((conversation, false));
            }

            DmConversationKind::Direct
        } else {
            let Some(title) = normalized_title.as_deref() else {
                return Err(ServiceError::ValidationError(
                    "Group conversations require a title".to_string(),
                ));
            };

            if title.chars().count() > 100 {
                return Err(ServiceError::ValidationError(
                    "Conversation title must not exceed 100 characters".to_string(),
                ));
            }

            if let Some(max_members) = self.repository.current_group_member_limit().await? {
                if participant_ids.len() > max_members as usize {
                    return Err(ServiceError::ValidationError(format!(
                        "Group conversations cannot exceed {} members",
                        max_members
                    )));
                }
            }

            DmConversationKind::Group
        };

        let conversation = self
            .repository
            .create_conversation(
                creator_id,
                &participant_ids,
                kind,
                normalized_title.as_deref(),
            )
            .await?;

        let response = self
            .get_conversation(conversation.conversation_id, creator_id)
            .await?;

        Ok((response, true))
    }

    /// Lists DM conversations for the authenticated user.
    pub async fn list_conversations(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DmConversationSummaryResponse>, ServiceError> {
        let conversations = self
            .repository
            .list_conversations(user_id, limit, offset)
            .await?;
        let conversation_ids = conversations
            .iter()
            .map(|conversation| conversation.conversation_id)
            .collect::<Vec<_>>();

        let participants = self
            .repository
            .list_participants_for_conversations(&conversation_ids)
            .await?;
        let latest_messages = self
            .repository
            .list_latest_messages_for_conversations(&conversation_ids)
            .await?;
        let unread_counts = self
            .repository
            .list_unread_counts(user_id, &conversation_ids)
            .await?;

        let participants_by_conversation = Self::group_participants(participants);
        let latest_messages_by_conversation = latest_messages
            .into_iter()
            .map(|message| (message.conversation_id, message))
            .collect::<HashMap<_, _>>();
        let unread_counts_by_conversation = unread_counts
            .into_iter()
            .map(|record| (record.conversation_id, record.unread_count))
            .collect::<HashMap<_, _>>();

        let mut responses = Vec::with_capacity(conversations.len());
        for conversation in conversations {
            let participant_records = participants_by_conversation
                .get(&conversation.conversation_id)
                .cloned()
                .unwrap_or_default();
            let participant_responses = participant_records
                .iter()
                .cloned()
                .map(DmConversationParticipantResponse::from)
                .collect::<Vec<_>>();
            let (display_title, direct_partner_id) = Self::display_title(
                &conversation.kind,
                conversation.title.as_deref(),
                &participant_records,
                user_id,
            );
            let last_message = latest_messages_by_conversation
                .get(&conversation.conversation_id)
                .cloned()
                .map(DmMessageResponse::from);
            let unread_count = unread_counts_by_conversation
                .get(&conversation.conversation_id)
                .copied()
                .unwrap_or(0);

            responses.push(DmConversationSummaryResponse {
                conversation_id: conversation.conversation_id,
                kind: conversation.kind,
                title: conversation.title,
                display_title,
                direct_partner_id,
                created_by: conversation.created_by,
                created_at: conversation.created_at,
                updated_at: conversation.updated_at,
                last_activity_at: conversation.last_activity_at,
                participant_count: conversation.participant_count,
                unread_count,
                participants: participant_responses,
                last_message,
            });
        }

        Ok(responses)
    }

    /// Retrieves a DM conversation detail for the authenticated user.
    pub async fn get_conversation(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<DmConversationResponse, ServiceError> {
        self.ensure_conversation_member(conversation_id, user_id)
            .await?;

        let conversation = self
            .repository
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or(ServiceError::NotFound)?;
        let participants = self
            .repository
            .list_participants_for_conversations(&[conversation_id])
            .await?;
        let latest_messages = self
            .repository
            .list_latest_messages_for_conversations(&[conversation_id])
            .await?;
        let unread_counts = self
            .repository
            .list_unread_counts(user_id, &[conversation_id])
            .await?;

        let (display_title, direct_partner_id) = Self::display_title(
            &conversation.kind,
            conversation.title.as_deref(),
            &participants,
            user_id,
        );
        let last_message = latest_messages
            .into_iter()
            .next()
            .map(DmMessageResponse::from);
        let unread_count = unread_counts
            .into_iter()
            .next()
            .map(|record| record.unread_count)
            .unwrap_or(0);

        Ok(DmConversationResponse {
            conversation_id: conversation.conversation_id,
            kind: conversation.kind,
            title: conversation.title,
            display_title,
            direct_partner_id,
            created_by: conversation.created_by,
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
            participant_count: participants.len() as i64,
            unread_count,
            participants: participants
                .into_iter()
                .map(DmConversationParticipantResponse::from)
                .collect(),
            last_message,
        })
    }

    /// Persists a new DM message and broadcasts it to active conversation participants.
    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: String,
    ) -> Result<DmMessageResponse, ServiceError> {
        Self::validate_message_content(&content)?;
        self.ensure_conversation_member(conversation_id, sender_id)
            .await?;

        let message = self
            .repository
            .send_message(conversation_id, sender_id, &content)
            .await?;
        let message = self
            .repository
            .find_message_record(message.message_id)
            .await?
            .ok_or(ServiceError::NotFound)?;
        let response = DmMessageResponse::from(message);

        if let Err(error) = self.broadcast_new_message(conversation_id, &response).await {
            warn!(
                conversation_id = %conversation_id,
                message_id = %response.message_id,
                error = ?error,
                "failed to broadcast DM message"
            );
        }

        Ok(response)
    }

    /// Lists messages for a DM conversation after validating access.
    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        before_message_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<DmMessageResponse>, ServiceError> {
        self.ensure_conversation_member(conversation_id, user_id)
            .await?;

        if let Some(before_message_id) = before_message_id {
            let belongs = self
                .repository
                .message_belongs_to_conversation(before_message_id, conversation_id)
                .await?;
            if !belongs {
                return Err(ServiceError::NotFound);
            }
        }

        let messages = self
            .repository
            .list_messages(conversation_id, before_message_id, limit)
            .await?;

        Ok(messages.into_iter().map(DmMessageResponse::from).collect())
    }

    /// Updates the authenticated user's read cursor for a conversation.
    pub async fn mark_as_read(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        up_to_message_id: Option<Uuid>,
    ) -> Result<(), ServiceError> {
        self.ensure_conversation_member(conversation_id, user_id)
            .await?;

        if let Some(up_to_message_id) = up_to_message_id {
            let belongs = self
                .repository
                .message_belongs_to_conversation(up_to_message_id, conversation_id)
                .await?;
            if !belongs {
                return Err(ServiceError::NotFound);
            }
        }

        if self
            .repository
            .mark_as_read(conversation_id, user_id, up_to_message_id)
            .await?
        {
            Ok(())
        } else {
            Err(ServiceError::NotFound)
        }
    }

    /// Soft-deletes a DM message authored by the authenticated user.
    pub async fn delete_message(
        &self,
        message_id: Uuid,
        requesting_user_id: Uuid,
    ) -> Result<(), ServiceError> {
        let message = self
            .repository
            .find_message_by_id(message_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        self.ensure_conversation_member(message.conversation_id, requesting_user_id)
            .await?;

        if message.sender_id != requesting_user_id {
            return Err(ServiceError::Forbidden);
        }

        if message.deleted_at.is_some() {
            return Err(ServiceError::ValidationError(
                "Message has already been deleted".to_string(),
            ));
        }

        if self.repository.soft_delete_message(message_id).await? {
            Ok(())
        } else {
            Err(ServiceError::ValidationError(
                "Message has already been deleted".to_string(),
            ))
        }
    }

    /// Returns whether the provided conversation listing parameters are valid.
    pub fn validate_conversation_pagination(limit: i64, offset: i64) -> Result<(), ServiceError> {
        if !(1..=100).contains(&limit) {
            return Err(ServiceError::ValidationError(
                "Conversation page size must be between 1 and 100".to_string(),
            ));
        }

        if offset < 0 {
            return Err(ServiceError::ValidationError(
                "Conversation offset must be zero or greater".to_string(),
            ));
        }

        Ok(())
    }

    /// Returns whether the provided message listing parameters are valid.
    pub fn validate_message_pagination(limit: i64) -> Result<(), ServiceError> {
        if !(1..=100).contains(&limit) {
            return Err(ServiceError::ValidationError(
                "Message page size must be between 1 and 100".to_string(),
            ));
        }

        Ok(())
    }

    async fn ensure_conversation_member(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ServiceError> {
        let access = self
            .repository
            .get_conversation_access(conversation_id, user_id)
            .await?;

        if !access.conversation_exists {
            return Err(ServiceError::NotFound);
        }

        if !access.is_member {
            return Err(ServiceError::Forbidden);
        }

        Ok(())
    }

    async fn ensure_users_exist(&self, participant_ids: &[Uuid]) -> Result<(), ServiceError> {
        for participant_id in participant_ids {
            let exists = self
                .user_repository
                .find_active_by_id(*participant_id)
                .await?
                .is_some();

            if !exists {
                return Err(ServiceError::NotFound);
            }
        }

        Ok(())
    }

    async fn ensure_no_blocked_pairs(&self, participant_ids: &[Uuid]) -> Result<(), ServiceError> {
        for (index, left_user_id) in participant_ids.iter().enumerate() {
            for right_user_id in participant_ids.iter().skip(index + 1) {
                if self
                    .block_repository
                    .exists_between(*left_user_id, *right_user_id)
                    .await?
                {
                    return Err(ServiceError::ValidationError(
                        "Cannot create a conversation between blocked users".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn broadcast_new_message(
        &self,
        conversation_id: Uuid,
        message: &DmMessageResponse,
    ) -> Result<(), ServiceError> {
        let participant_ids = self
            .repository
            .list_participant_user_ids(conversation_id)
            .await?;
        let senders = {
            let subscribers = self.presence_tx.lock().await;
            participant_ids
                .iter()
                .filter_map(|participant_id| subscribers.get(participant_id).cloned())
                .collect::<Vec<_>>()
        };

        if senders.is_empty() {
            return Ok(());
        }

        let payload = ServerWsMessage::NewMessage {
            conversation_id,
            message: message.clone(),
        };
        let json = serde_json::to_string(&payload).map_err(|error| {
            ServiceError::ValidationError(format!("Failed to serialize DM event: {}", error))
        })?;

        for sender in senders {
            let _ = sender.send(json.clone());
        }

        Ok(())
    }

    fn normalize_participant_ids(&self, creator_id: Uuid, participant_ids: &[Uuid]) -> Vec<Uuid> {
        let mut participant_ids = participant_ids
            .iter()
            .copied()
            .chain(std::iter::once(creator_id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        participant_ids.sort_unstable();
        participant_ids
    }

    fn normalize_title(title: Option<String>) -> Option<String> {
        title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty())
    }

    fn validate_message_content(content: &str) -> Result<(), ServiceError> {
        if content.trim().is_empty() {
            return Err(ServiceError::ValidationError(
                "Message content cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn display_title(
        kind: &DmConversationKind,
        stored_title: Option<&str>,
        participants: &[DmConversationParticipantRecord],
        current_user_id: Uuid,
    ) -> (String, Option<Uuid>) {
        if *kind == DmConversationKind::Direct {
            let direct_partner = participants
                .iter()
                .find(|participant| participant.user_id != current_user_id)
                .or_else(|| participants.first());

            return match direct_partner {
                Some(direct_partner) => (
                    direct_partner.username.clone(),
                    Some(direct_partner.user_id),
                ),
                None => ("Direct Message".to_string(), None),
            };
        }

        match stored_title {
            Some(title) => (title.to_string(), None),
            None => (
                participants
                    .iter()
                    .map(|participant| participant.username.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                None,
            ),
        }
    }

    fn group_participants(
        participants: Vec<DmConversationParticipantRecord>,
    ) -> HashMap<Uuid, Vec<DmConversationParticipantRecord>> {
        let mut grouped = HashMap::<Uuid, Vec<DmConversationParticipantRecord>>::new();
        for participant in participants {
            grouped
                .entry(participant.conversation_id)
                .or_default()
                .push(participant);
        }
        grouped
    }
}
