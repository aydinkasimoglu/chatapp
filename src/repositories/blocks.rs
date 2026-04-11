use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{BlockRecord, UserBlock};

/// Data access object for user-block operations.
///
/// Handles all database queries related to blocking one user from another.
/// Blocks are directional: `blocker_id` initiated the block against `blocked_id`.
#[derive(Clone)]
pub struct BlockRepository {
    pool: PgPool,
}

impl BlockRepository {
    /// Creates a new `BlockRepository` instance.
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inserts a new block record from `blocker_id` targeting `blocked_id`.
    ///
    /// Returns a unique-constraint database error if `blocker_id` has already
    /// blocked `blocked_id`.
    pub async fn create(
        &self,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<UserBlock, sqlx::Error> {
        sqlx::query_as!(
            UserBlock,
            r#"
            INSERT INTO user_blocks (blocker_id, blocked_id)
            VALUES ($1, $2)
            RETURNING block_id, blocker_id, blocked_id, created_at
            "#,
            blocker_id,
            blocked_id,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Looks up a specific directed block (`blocker_id` → `blocked_id`).
    ///
    /// Returns `None` if no such block exists.
    #[allow(dead_code)]
    pub async fn find(
        &self,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<Option<UserBlock>, sqlx::Error> {
        sqlx::query_as!(
            UserBlock,
            r#"
            SELECT block_id, blocker_id, blocked_id, created_at
            FROM user_blocks
            WHERE blocker_id = $1 AND blocked_id = $2
            "#,
            blocker_id,
            blocked_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Returns true if either user has blocked the other.
    ///
    /// This is used to gate social actions (such as sending friend requests)
    /// whenever a block exists in either direction.
    pub async fn exists_between(&self, user_a: Uuid, user_b: Uuid) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM user_blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            ) as "exists!"
            "#,
            user_a,
            user_b,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// Removes the directed block from `blocker_id` → `blocked_id`.
    ///
    /// Returns `true` if a row was deleted, `false` if no matching block was found.
    pub async fn delete(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query!(
                "DELETE FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2",
                blocker_id,
                blocked_id
            )
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Retrieves all users blocked by `blocker_id`, joined with their profile data.
    ///
    /// Deactivated users are excluded. Results are ordered newest-first.
    pub async fn list_blocked_by(&self, blocker_id: Uuid) -> Result<Vec<BlockRecord>, sqlx::Error> {
        sqlx::query_as!(
            BlockRecord,
            r#"
            SELECT
                ub.block_id,
                u.user_id,
                u.username,
                u.email,
                ub.created_at
            FROM user_blocks ub
            JOIN users u ON u.user_id = ub.blocked_id
            WHERE ub.blocker_id = $1
              AND u.is_active = TRUE
            ORDER BY ub.created_at DESC
            "#,
            blocker_id,
        )
        .fetch_all(&self.pool)
        .await
    }
}
