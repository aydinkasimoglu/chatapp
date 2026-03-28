use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::RefreshToken;

#[derive(Clone)]
pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores a new hashed refresh token for the given user.
    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
             VALUES ($1, $2, $3)",
            user_id,
            token_hash,
            expires_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Looks up an **unexpired** token record by its SHA-256 hex hash.
    ///
    /// # Fix
    /// The query now filters `expires_at > NOW()` so callers are never handed
    /// a stale record. Previously an expired token would be returned and the
    /// expiry check lived entirely in the service layer, which was a silent
    /// footgun for any future caller that forgot to re-check.
    pub async fn find_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as!(
            RefreshToken,
            "SELECT token_id, user_id
             FROM refresh_tokens
             WHERE token_hash = $1
               AND expires_at > NOW()",
            token_hash,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a single token by its primary key (used during rotation).
    pub async fn delete(&self, token_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE token_id = $1", token_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes a single token by hash (used for single-device logout).
    pub async fn delete_by_hash(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE token_hash = $1",
            token_hash
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes all tokens for a user (used for logout-all-devices or account deactivation).
    pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = $1", user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Keeps only the `keep_n` most recently created tokens for a user,
    /// deleting any older ones.
    ///
    /// Call this **before** inserting a new token, passing
    /// `MAX_REFRESH_TOKENS_PER_USER - 1` so that after the insert the total
    /// stays within the cap.
    ///
    /// The inner subquery selects the `keep_n` newest token IDs for the user;
    /// the outer DELETE removes every other row belonging to that user. This
    /// is a single round-trip and is safe under concurrent inserts because
    /// the worst case is a temporary overshoot by one row, corrected on the
    /// next login.
    pub async fn prune_oldest_for_user(
        &self,
        user_id: Uuid,
        keep_n: usize,
    ) -> Result<(), sqlx::Error> {
        let keep_n = keep_n as i64;
        sqlx::query!(
            "DELETE FROM refresh_tokens
             WHERE user_id = $1
               AND token_id NOT IN (
                   SELECT token_id
                   FROM refresh_tokens
                   WHERE user_id = $1
                   ORDER BY created_at DESC
                   LIMIT $2
               )",
            user_id,
            keep_n,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes all expired tokens across every user.
    ///
    /// This is **not** called in the hot path. Schedule it as a periodic
    /// maintenance task (e.g. a nightly cron or background Tokio task) to
    /// prevent the table from growing unboundedly from tokens that expired
    /// without an explicit logout or rotation.
    ///
    /// Returns the number of rows deleted.
    pub async fn delete_all_expired(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM refresh_tokens WHERE expires_at <= NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}