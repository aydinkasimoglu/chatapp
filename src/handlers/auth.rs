use crate::{
    error::ServiceError,
    models::{AuthRequest, AuthResponse, CreateUser, UserResponse},
    state::AppState,
};

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

/// Authenticates a user and returns a JWT token.
///
/// Accepts email and password credentials, verifies them against the database,
/// and returns a JWT token for authenticated requests.
///
/// # Arguments
/// * `state` - Application state containing auth service
/// * `payload` - Authentication request with email and password
///
/// # Returns
/// A JWT token wrapped in an `AuthResponse` on success
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