use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{AuthRequest, AuthResponse, CreateUser, RefreshRequest, UserResponse},
    state::AppState,
};

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

/// Authenticates a user and returns a short-lived access token + opaque refresh token.
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ServiceError> {
    let response = state.auth_service.authenticate(payload).await?;
    Ok(Json(response))
}

/// Creates a new user account.
///
/// Registers a new user with the provided username and email.
/// Returns HTTP 201 Created on success.
///
/// # Arguments
/// * `state` - Application state containing user service
/// * `payload` - User creation data (username, email)
///
/// # Returns
/// HTTP 201 with the created user on success
pub async fn signup_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserResponse>), ServiceError> {
    let user = state.user_service.create(payload).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// Exchanges a valid refresh token for a new access token + rotated refresh token.
pub async fn refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, ServiceError> {
    let response = state.auth_service.refresh(payload).await?;
    Ok(Json(response))
}

/// Revokes the supplied refresh token (single-device logout).
/// If no body is supplied, revokes **all** tokens for the authenticated user.
pub async fn logout_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    payload: Option<Json<RefreshRequest>>,
) -> Result<StatusCode, ServiceError> {
    let raw_token = payload.as_ref().map(|Json(r)| r.refresh_token.as_str());
    state.auth_service.logout(user_id, raw_token).await?;
    Ok(StatusCode::NO_CONTENT)
}