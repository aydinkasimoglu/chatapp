use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateUser, UpdatePassword, UpdateUser, User};

/// Data access object for user operations.
///
/// Handles all database queries related to users. Uses SQLx for type-safe
/// SQL query execution.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Creates a new `UserRepository` instance.
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a new user in the database.
    ///
    /// Hashes the password with Argon2 before persisting.
    ///
    /// # Arguments
    /// * `payload` - User creation data
    ///
    /// # Returns
    /// The created user with all database-assigned fields
    pub async fn create(&self, payload: &CreateUser) -> Result<User, sqlx::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING
                user_id,
                username,
                email,
                password_hash,
                created_at,
                updated_at
            "#,
            &payload.username,
            &payload.email,
            &password_hash,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Retrieves a user by UUID.
    ///
    /// # Arguments
    /// * `user_id` - The user ID
    ///
    /// # Returns
    /// The user if found, or None if not found
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT user_id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_active_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT user_id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE user_id = $1 AND is_active = TRUE
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Retrieves a user by email address.
    ///
    /// # Arguments
    /// * `email` - The email address to search for
    ///
    /// # Returns
    /// The user if found, or None if not found
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT user_id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = TRUE
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Retrieves all users from the database.
    ///
    /// # Returns
    /// A vector of all users
    pub async fn find_all(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT user_id, username, email, password_hash, created_at, updated_at
            FROM users
            WHERE is_active = TRUE
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Updates a user's information.
    ///
    /// Supports partial updates - only provided fields are modified (username
    /// and email only). Passwords must be changed through the dedicated
    /// `update_password` method which handles hashing and verification.
    ///
    /// # Arguments
    /// * `id` - The user ID to update
    /// * `payload` - Update data with optional fields
    ///
    /// # Returns
    /// The updated user if found, or None if user doesn't exist
    pub async fn update(
        &self,
        user_id: Uuid,
        payload: &UpdateUser,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = COALESCE($1::text, username),
                email    = COALESCE($2::text, email)
            WHERE user_id = $3 AND is_active = TRUE
            RETURNING user_id, username, email, password_hash, created_at, updated_at
            "#,
            payload.username.as_deref(),
            payload.email.as_deref(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Verifies `current_password` against the stored hash before
    /// applying the new one. Returns `None` if the user doesn't exist,
    /// `Some(false)` if the current password is wrong, `Some(true)` on success.
    pub async fn update_password(
        &self,
        user_id: Uuid,
        payload: &UpdatePassword,
    ) -> Result<Option<bool>, sqlx::Error> {
        use argon2::{PasswordVerifier, password_hash::PasswordHash};

        // Fetch the current hash first
        let row = sqlx::query!(
            r#"
            SELECT password_hash FROM users
            WHERE user_id = $1 AND is_active = TRUE
            "#,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None); // user not found
        };

        // Verify current password
        let parsed_hash = PasswordHash::new(&row.password_hash).unwrap();
        let valid = Argon2::default()
            .verify_password(payload.current_password.as_bytes(), &parsed_hash)
            .is_ok();

        if !valid {
            return Ok(Some(false)); // wrong current password
        }

        // Hash and apply the new password
        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(payload.new_password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"
            UPDATE users SET password_hash = $1
            WHERE user_id = $2 AND is_active = TRUE
            "#,
            new_hash,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(Some(true))
    }

    /// Makes a user inactive in the database.
    ///
    /// # Arguments
    /// * `user_id` - The user ID to delete
    ///
    /// # Returns
    /// true if the operation is successful, false otherwise
    pub async fn deactivate(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET is_active = FALSE
            WHERE user_id = $1 AND is_active = TRUE
            "#,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
