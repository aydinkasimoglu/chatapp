use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{OnlineFriendRecord, PresenceStatus};

#[derive(Clone)]
pub struct PresenceRepository {
    pool: PgPool,
}

impl PresenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inserts a new session row when a WebSocket connection is established.
    /// Returns the DB-generated session UUID.
    pub async fn connect(&self, user_id: Uuid) -> Result<Uuid, sqlx::Error> {
        let row: Uuid = sqlx::query_scalar!(
            r#"
            INSERT INTO user_presence (user_id, status, last_heartbeat_at, connected_at)
            VALUES ($1, 'online', NOW(), NOW())
            RETURNING session_id
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Updates the heartbeat timestamp and status for an existing session.
    pub async fn heartbeat(
        &self,
        session_id: Uuid,
        status: PresenceStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE user_presence
            SET last_heartbeat_at = NOW(),
                status = $2::presence_status
            WHERE session_id = $1
            "#,
            session_id,
            status as _
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Removes a session row on clean WebSocket disconnect.
    pub async fn disconnect(&self, session_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM user_presence WHERE session_id = $1", session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes all sessions whose heartbeat is older than 30 seconds.
    /// Call this periodically from a background task to handle crashed clients.
    pub async fn cleanup_stale(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM user_presence WHERE last_heartbeat_at < NOW() - INTERVAL '30 seconds'",
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Returns accepted friends of `user_id` that have at least one fresh session.
    ///
    /// Aggregated across all sessions per user:
    ///   - status = 'online' if any session is 'online'
    ///   - status = 'idle'   if all sessions are 'idle'
    pub async fn online_friends(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OnlineFriendRecord>, sqlx::Error> {
        sqlx::query!(
            r#"
            WITH my_friends AS (
                SELECT
                    CASE
                        WHEN f.requester_id = $1 THEN f.addressee_id
                        ELSE f.requester_id
                    END AS friend_id
                FROM friendships f
                WHERE (f.requester_id = $1 OR f.addressee_id = $1)
                  AND f.status = 'accepted'
            )
            SELECT
                u.user_id,
                u.username,
                MAX(p.last_heartbeat_at) AS last_heartbeat_at,
                CASE WHEN bool_or(p.status = 'online') THEN 'online' ELSE 'idle' END AS status
            FROM my_friends mf
            JOIN users         u ON u.user_id  = mf.friend_id
            JOIN user_presence p ON p.user_id  = u.user_id
            WHERE p.last_heartbeat_at >= NOW() - INTERVAL '30 seconds'
            GROUP BY u.user_id, u.username
            ORDER BY u.username
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| OnlineFriendRecord {
                    user_id: row.user_id,
                    username: row.username,
                    last_heartbeat_at: row.last_heartbeat_at.unwrap_or_default(),
                    status: match row.status.as_deref() {
                        Some("online") => PresenceStatus::Online,
                        Some("idle") => PresenceStatus::Idle,
                        _ => PresenceStatus::Idle,
                    },
                })
                .collect::<Vec<_>>()
        })
    }

    /// Returns `true` if `user_id` has at least one active session.
    pub async fn has_active_sessions(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_presence WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }
}
