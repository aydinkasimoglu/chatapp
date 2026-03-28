use crate::{
    error::ServiceError,
    models::{CreateUser, UpdatePassword, UpdateUser, User},
    repositories::user::UserRepository,
};

use uuid::Uuid;

/// Service for managing user-related operations.
///
/// Provides CRUD operations for users including creation, retrieval, updates,
/// and deletion. Maps database errors to appropriate service errors.
#[derive(Clone)]
pub struct UserService {
    repository: UserRepository,
}

impl UserService {
    /// Creates a new `UserService` instance.
    ///
    /// # Arguments
    /// * `repository` - User repository for database operations
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    /// Creates a new user in the database.
    ///
    /// # Arguments
    /// * `payload` - User creation data (username, email)
    ///
    /// # Returns
    /// The created user on success, or a `ServiceError` (e.g., `DuplicateUser` if username/email exists)
    pub async fn create(&self, payload: CreateUser) -> Result<User, ServiceError> {
        self.repository
            .create(&payload)
            .await
            .map_err(|err| self.map_db_error(err))
    }

    /// Retrieves a user by their ID.
    ///
    /// # Arguments
    /// * `user_id` - The user ID to retrieve
    ///
    /// # Returns
    /// The user on success, or `NotFound` error if user doesn't exist
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<User, ServiceError> {
        self.repository
            .find_by_id(user_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Retrieves a page of users.
    ///
    /// # Arguments
    /// * `limit`  - Maximum number of users to return (caller should cap this)
    /// * `offset` - Number of users to skip
    ///
    /// # Returns
    /// A vector of users for the requested page
    pub async fn find_paginated(&self, limit: i64, offset: i64) -> Result<Vec<User>, ServiceError> {
        Ok(self.repository.find_paginated(limit, offset).await?)
    }

    /// Updates a user's information.
    ///
    /// Supports partial updates - only provided fields are modified.
    ///
    /// # Arguments
    /// * `user_id` - The user ID to update
    /// * `payload` - Update data with optional username and email
    ///
    /// # Returns
    /// The updated user on success, or `NotFound` error if user doesn't exist
    pub async fn update(
        &self,
        user_id: Uuid,
        payload: UpdateUser,
    ) -> Result<User, ServiceError> {
        self.repository
            .update(user_id, &payload)
            .await
            .map_err(|err| self.map_db_error(err))?
            .ok_or(ServiceError::NotFound)
    }

    /// Delegates all hashing and verification to the repository.
    /// Interprets the `Option<bool>` result:
    ///   None        → user not found
    ///   Some(false) → current password incorrect
    ///   Some(true)  → success
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: String,
        new_password: String,
    ) -> Result<(), ServiceError> {
        let payload = UpdatePassword {
            current_password,
            new_password,
        };

        match self.repository.update_password(user_id, &payload).await? {
            None => Err(ServiceError::NotFound),
            Some(false) => Err(ServiceError::Unauthorized),
            Some(true) => Ok(()),
        }
    }

    /// Soft-deletes the user by setting `is_active = FALSE`.
    pub async fn deactivate(&self, user_id: Uuid) -> Result<(), ServiceError> {
        match self.repository.deactivate(user_id).await? {
            true => Ok(()),
            false => Err(ServiceError::NotFound),
        }
    }

    /// Maps database errors to service errors.
    ///
    /// Translates specific database constraint violations to appropriate
    /// service errors (e.g., duplicate username/email to `DuplicateUser`).
    ///
    /// # Arguments
    /// * `err` - The database error to map
    ///
    /// # Returns
    /// A `ServiceError` appropriate for the database error
    fn map_db_error(&self, err: sqlx::Error) -> ServiceError {
        if let Some(db_error) = err.as_database_error() {
            if let Some(constraint) = db_error.constraint() {
                if constraint == "uq_users_username" || constraint == "uq_users_email" {
                    return ServiceError::DuplicateUser;
                }
            }
        }
        ServiceError::Database(err)
    }
}
