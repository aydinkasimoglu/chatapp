use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{
        CreateDmConversation, CursorPaginatedResponse, DmConversationListQuery,
        DmConversationResponse, DmConversationSummaryResponse, DmMessageListQuery,
        DmMessageResponse, MarkDmConversationRead, PaginatedResponse, SendDmMessage,
    },
    state::AppState,
};

/// Creates a direct conversation or group DM for the authenticated user.
pub async fn create_conversation_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateDmConversation>,
) -> Result<(StatusCode, Json<DmConversationResponse>), ServiceError> {
    let (conversation, created) = state.dm_service.create_conversation(user_id, payload).await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, Json(conversation)))
}

/// Lists DM conversations for the authenticated user.
pub async fn list_conversations_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Query(params): Query<DmConversationListQuery>,
) -> Result<Json<PaginatedResponse<DmConversationSummaryResponse>>, ServiceError> {
    const DEFAULT_LIMIT: i64 = 50;

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    let offset = params.offset.unwrap_or(0);
    crate::services::dm::DmService::validate_conversation_pagination(limit, offset)?;

    let conversations = state.dm_service.list_conversations(user_id, limit, offset).await?;
    Ok(Json(PaginatedResponse {
        items: conversations,
        limit,
        offset,
    }))
}

/// Returns the DM conversation detail for the authenticated user.
pub async fn get_conversation_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<DmConversationResponse>, ServiceError> {
    let conversation = state.dm_service.get_conversation(conversation_id, user_id).await?;
    Ok(Json(conversation))
}

/// Persists a new message in a DM conversation.
pub async fn send_message_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<SendDmMessage>,
) -> Result<(StatusCode, Json<DmMessageResponse>), ServiceError> {
    let message = state
        .dm_service
        .send_message(conversation_id, user_id, payload.content)
        .await?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// Lists messages in a DM conversation using a descending cursor.
pub async fn list_messages_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(conversation_id): Path<Uuid>,
    Query(params): Query<DmMessageListQuery>,
) -> Result<Json<CursorPaginatedResponse<DmMessageResponse>>, ServiceError> {
    const DEFAULT_LIMIT: i64 = 50;

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    crate::services::dm::DmService::validate_message_pagination(limit)?;

    let messages = state
        .dm_service
        .list_messages(conversation_id, user_id, params.before_message_id, limit)
        .await?;
    let next_before_message_id = messages.last().map(|message| message.message_id);

    Ok(Json(CursorPaginatedResponse {
        items: messages,
        limit,
        before_message_id: params.before_message_id,
        next_before_message_id,
    }))
}

/// Marks a DM conversation as read up to a specific message.
pub async fn mark_as_read_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<MarkDmConversationRead>,
) -> Result<StatusCode, ServiceError> {
    state
        .dm_service
        .mark_as_read(conversation_id, user_id, payload.up_to_message_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Soft-deletes a DM message authored by the authenticated user.
pub async fn delete_message_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(message_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state.dm_service.delete_message(message_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}