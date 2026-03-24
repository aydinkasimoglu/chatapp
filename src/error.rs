use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::models::ErrorResponse;

/// Errors that can occur during service operations.
///
/// Represents all possible errors that may be returned by service operations,
/// with automatic HTTP status code mapping via the IntoResponse implementation.
#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("User with this username or email already exists")]
    DuplicateUser,
    #[error("Users are already friends")]
    AlreadyFriends,
    #[error("A friend request is already pending for these users")]
    FriendRequestAlreadyPending,
    #[error("This friend request can no longer be updated")]
    InvalidFriendRequestState,
    #[error("Cannot perform this action because one user has blocked the other")]
    BlockedRelationship,
    #[error("You have already blocked this user")]
    AlreadyBlocked,
    #[error("User not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Token generation failed")]
    JWTGenFailed,
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ServiceError::ValidationError(msg) => create_error(StatusCode::BAD_REQUEST, msg),
            ServiceError::DuplicateUser => create_error(StatusCode::CONFLICT, self.to_string()),
            ServiceError::AlreadyFriends => create_error(StatusCode::CONFLICT, self.to_string()),
            ServiceError::FriendRequestAlreadyPending => {
                create_error(StatusCode::CONFLICT, self.to_string())
            }
            ServiceError::InvalidFriendRequestState => {
                create_error(StatusCode::CONFLICT, self.to_string())
            }
            ServiceError::BlockedRelationship => {
                create_error(StatusCode::CONFLICT, self.to_string())
            }
            ServiceError::AlreadyBlocked => create_error(StatusCode::CONFLICT, self.to_string()),
            ServiceError::NotFound => create_error(StatusCode::NOT_FOUND, self.to_string()),
            ServiceError::Unauthorized => create_error(StatusCode::UNAUTHORIZED, self.to_string()),
            ServiceError::InvalidToken => create_error(StatusCode::UNAUTHORIZED, self.to_string()),
            ServiceError::Database(err) => {
                eprintln!("Database error: {:?}", err);
                create_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ServiceError::JWTGenFailed => {
                create_error(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        (status, error_message).into_response()
    }
}

/// Creates an error response with the given status code and message.
///
/// Helper function to standardize error response formatting.
fn create_error(status: StatusCode, message: String) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: message }))
}
