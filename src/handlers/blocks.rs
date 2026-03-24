use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    error::ServiceError, extractors::AuthenticatedUser, models::BlockResponse, state::AppState,
};

/// Blocks the specified user on behalf of the authenticated user.
///
/// Any existing friendship or pending request between the two users is removed
/// automatically before the block is created. Returns HTTP 201 Created with the
/// new block record on success.
///
/// # Errors
/// - `400 Bad Request` if the caller tries to block themselves.
/// - `404 Not Found` if the target user does not exist or is deactivated.
/// - `409 Conflict` if the target user is already blocked.
pub async fn block_user_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<(StatusCode, Json<BlockResponse>), ServiceError> {
    let block = state
        .block_service
        .block_user(user_id, target_user_id)
        .await?;
    Ok((StatusCode::CREATED, Json(block)))
}

/// Removes the block the authenticated user placed on the specified user.
///
/// After unblocking either user may send a new friend request. Returns HTTP 204
/// No Content on success.
///
/// # Errors
/// - `404 Not Found` if the authenticated user has not blocked the target user.
pub async fn unblock_user_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state
        .block_service
        .unblock_user(user_id, target_user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Returns the list of all users that the authenticated user has currently blocked.
///
/// Deactivated accounts are excluded from the results. Results are ordered
/// newest-first.
pub async fn get_blocked_users_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<BlockResponse>>, ServiceError> {
    let blocked = state.block_service.list_blocked(user_id).await?;
    Ok(Json(blocked))
}
