use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{
        FriendRequestPayload, FriendResponse, FriendshipResponse, PendingFriendRequestResponse,
    },
    state::AppState,
};

/// Sends a new friend request from the authenticated user to `addressee_id`.
///
/// Returns HTTP 201 Created and the resulting friendship row in `pending` status.
pub async fn send_friend_request_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<FriendRequestPayload>,
) -> Result<(StatusCode, Json<FriendshipResponse>), ServiceError> {
    let friendship = state
        .friendship_service
        .send_request(user_id, payload.addressee_id)
        .await?;

    Ok((StatusCode::CREATED, Json(friendship)))
}

/// Returns all accepted friends for the authenticated user.
pub async fn get_friends_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<FriendResponse>>, ServiceError> {
    let friends = state.friendship_service.list_friends(user_id).await?;
    Ok(Json(friends))
}

/// Returns pending incoming friend requests for the authenticated user.
pub async fn get_incoming_friend_requests_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<PendingFriendRequestResponse>>, ServiceError> {
    let requests = state
        .friendship_service
        .list_incoming_pending(user_id)
        .await?;
    Ok(Json(requests))
}

/// Returns pending outgoing friend requests sent by the authenticated user.
pub async fn get_outgoing_friend_requests_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<PendingFriendRequestResponse>>, ServiceError> {
    let requests = state
        .friendship_service
        .list_outgoing_pending(user_id)
        .await?;
    Ok(Json(requests))
}

/// Accepts a pending friend request.
///
/// Only the request addressee may perform this action.
pub async fn accept_friend_request_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(friendship_id): Path<Uuid>,
) -> Result<Json<FriendshipResponse>, ServiceError> {
    let friendship = state
        .friendship_service
        .accept_request(user_id, friendship_id)
        .await?;

    Ok(Json(friendship))
}

/// Rejects a pending friend request.
///
/// Only the request addressee may perform this action.
pub async fn reject_friend_request_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(friendship_id): Path<Uuid>,
) -> Result<Json<FriendshipResponse>, ServiceError> {
    let friendship = state
        .friendship_service
        .reject_request(user_id, friendship_id)
        .await?;

    Ok(Json(friendship))
}

/// Cancels an outgoing pending request created by the authenticated user.
///
/// Returns HTTP 204 No Content on success.
pub async fn cancel_friend_request_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(friendship_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state
        .friendship_service
        .cancel_request(user_id, friendship_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Removes an accepted friend relationship for the authenticated user.
///
/// Either side of an accepted friendship can remove it. Returns HTTP 204 No Content.
pub async fn remove_friend_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(friendship_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state
        .friendship_service
        .remove_friend(user_id, friendship_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
