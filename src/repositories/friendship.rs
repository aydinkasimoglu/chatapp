use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    FriendRecord, Friendship, PendingFriendRequestRecord,
};

#[derive(Clone)]
pub struct FriendshipRepository {
    pool: PgPool,
}

impl FriendshipRepository {
    /// Creates a new `FriendshipRepository` instance.
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a new pending friend request from requester to addressee.
    pub async fn create_request(
        &self,
        requester_id: Uuid,
        addressee_id: Uuid,
    ) -> Result<Friendship, sqlx::Error> {
        sqlx::query_as::<_, Friendship>(
            r#"
            INSERT INTO friendships (requester_id, addressee_id)
            VALUES ($1, $2)
            RETURNING friendship_id, requester_id, addressee_id, status, responded_at, created_at, updated_at
            "#,
        )
        .bind(requester_id)
        .bind(addressee_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Finds a friendship row by its identifier.
    pub async fn find_by_id(&self, friendship_id: Uuid) -> Result<Option<Friendship>, sqlx::Error> {
        sqlx::query_as::<_, Friendship>(
            r#"
            SELECT friendship_id, requester_id, addressee_id, status, responded_at, created_at, updated_at
            FROM friendships
            WHERE friendship_id = $1
            "#,
        )
        .bind(friendship_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Finds a friendship row between two users regardless of direction.
    pub async fn find_between(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Option<Friendship>, sqlx::Error> {
        sqlx::query_as::<_, Friendship>(
            r#"
            SELECT friendship_id, requester_id, addressee_id, status, responded_at, created_at, updated_at
            FROM friendships
            WHERE (requester_id = $1 AND addressee_id = $2)
               OR (requester_id = $2 AND addressee_id = $1)
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_optional(&self.pool)
        .await
    }

    /// Reopens an existing relationship row as a fresh pending request.
    ///
    /// This is used after a previously rejected request when one user
    /// sends a new request.
    pub async fn reopen_request(
        &self,
        friendship_id: Uuid,
        requester_id: Uuid,
        addressee_id: Uuid,
    ) -> Result<Friendship, sqlx::Error> {
        sqlx::query_as::<_, Friendship>(
            r#"
            UPDATE friendships
            SET requester_id = $1,
                addressee_id = $2,
                status = 'pending',
                responded_at = NULL
            WHERE friendship_id = $3
            RETURNING friendship_id, requester_id, addressee_id, status, responded_at, created_at, updated_at
            "#,
        )
        .bind(requester_id)
        .bind(addressee_id)
        .bind(friendship_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Updates a pending friendship to a final status.
    ///
    /// The row is only updated if the current status is `pending`.
    pub async fn update_status(
        &self,
        friendship_id: Uuid,
        status: &str,
    ) -> Result<Option<Friendship>, sqlx::Error> {
        sqlx::query_as::<_, Friendship>(
            r#"
            UPDATE friendships
            SET status = $1::friendship_status,
                responded_at = NOW()
            WHERE friendship_id = $2
              AND status = 'pending'
            RETURNING friendship_id, requester_id, addressee_id, status, responded_at, created_at, updated_at
            "#,
        )
        .bind(status)
        .bind(friendship_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a pending request where `requester_id` is the owner of that request.
    ///
    /// Returns `true` if a pending request was deleted.
    pub async fn delete_pending_request(
        &self,
        friendship_id: Uuid,
        requester_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM friendships
            WHERE friendship_id = $1
              AND requester_id = $2
              AND status = 'pending'
            "#,
        )
        .bind(friendship_id)
        .bind(requester_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deletes an accepted friendship for a participant.
    ///
    /// Returns `true` if the friendship existed and was removed.
    pub async fn delete_accepted_friendship(
        &self,
        friendship_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM friendships
            WHERE friendship_id = $1
              AND status = 'accepted'
              AND (requester_id = $2 OR addressee_id = $2)
            "#,
        )
        .bind(friendship_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deletes any relationship rows between two users regardless of status.
    ///
    /// Used by block operations to ensure there are no friendships or pending requests
    /// remaining after a block is created.
    pub async fn delete_between(&self, user_a: Uuid, user_b: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM friendships
            WHERE (requester_id = $1 AND addressee_id = $2)
               OR (requester_id = $2 AND addressee_id = $1)
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Lists accepted friends for the specified user.
    pub async fn list_friends(&self, user_id: Uuid) -> Result<Vec<FriendRecord>, sqlx::Error> {
        sqlx::query_as::<_, FriendRecord>(
            r#"
            SELECT
                f.friendship_id,
                CASE WHEN f.requester_id = $1 THEN other_user.user_id ELSE requester.user_id END AS user_id,
                CASE WHEN f.requester_id = $1 THEN other_user.username ELSE requester.username END AS username,
                CASE WHEN f.requester_id = $1 THEN other_user.email ELSE requester.email END AS email,
                COALESCE(f.responded_at, f.updated_at) AS friends_since
            FROM friendships f
            JOIN users requester ON requester.user_id = f.requester_id
            JOIN users other_user ON other_user.user_id = f.addressee_id
            WHERE f.status = 'accepted'
              AND (f.requester_id = $1 OR f.addressee_id = $1)
              AND requester.is_active = TRUE
              AND other_user.is_active = TRUE
            ORDER BY friends_since DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Lists incoming pending requests (requests this user can accept/reject).
    pub async fn list_incoming_pending(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PendingFriendRequestRecord>, sqlx::Error> {
        sqlx::query_as::<_, PendingFriendRequestRecord>(
            r#"
            SELECT
                f.friendship_id,
                u.user_id,
                u.username,
                u.email,
                f.status,
                f.created_at
            FROM friendships f
            JOIN users u ON u.user_id = f.requester_id
            WHERE f.addressee_id = $1
              AND f.status = 'pending'
              AND u.is_active = TRUE
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Lists outgoing pending requests sent by the specified user.
    pub async fn list_outgoing_pending(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PendingFriendRequestRecord>, sqlx::Error> {
        sqlx::query_as::<_, PendingFriendRequestRecord>(
            r#"
            SELECT
                f.friendship_id,
                u.user_id,
                u.username,
                u.email,
                f.status,
                f.created_at
            FROM friendships f
            JOIN users u ON u.user_id = f.addressee_id
            WHERE f.requester_id = $1
              AND f.status = 'pending'
              AND u.is_active = TRUE
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }
}
