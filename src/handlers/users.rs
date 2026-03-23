use crate::{
    error::ServiceError,
    models::{UpdatePassword, UpdateUser, UserResponse},
    state::AppState,
};

use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
    http::StatusCode,
};
use uuid::Uuid;

/// Updates an existing user's information.
///
/// Updates the username and/or email for the specified user.
/// Only provided fields are updated (partial updates supported).
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to update
/// * `payload` - Update data with optional username and email
///
/// # Returns
/// The updated user on success
pub async fn update_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, ServiceError> {
    let user = state.user_service.update(user_id, payload).await?;
    Ok(Json(user.into()))
}

/// Deactivates a user by UUID.
///
/// Removes the specified user from the database.
/// Returns HTTP 204 No Content on success.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to delete
///
/// # Returns
/// HTTP 204 No Content on success
pub async fn deactivate_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    state.user_service.deactivate(user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Changes a user's password.
///
/// Requires the current password for verification. Returns 204 No Content
/// on success.
pub async fn update_password_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdatePassword>,
) -> Result<StatusCode, ServiceError> {
    state
        .user_service
        .change_password(user_id, payload.current_password, payload.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Retrieves all users from the database.
///
/// Fetches a list of all registered users.
///
/// # Arguments
/// * `state` - Application state containing user service
///
/// # Returns
/// A vector of all users
pub async fn get_users_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, ServiceError> {
    let users = state.user_service.find_all().await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

/// Retrieves a specific user by ID.
///
/// Fetches user information for the given UUID.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `user_id` - UUID to retrieve
///
/// # Returns
/// The requested user on success, or NotFound error if user doesn't exist
pub async fn get_user_by_id_handler(
    State(state): State<AppState>,
    path: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<UserResponse>, ServiceError> {
    let Path(user_id) = path.map_err(|_| ServiceError::ValidationError(
        "Invalid user ID format, expected a UUID".to_string()
    ))?;
    let user = state.user_service.find_by_id(user_id).await?;
    Ok(Json(user.into()))
}
