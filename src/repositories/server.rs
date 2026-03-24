use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateServer, Server, UpdateServer};

#[derive(Clone)]
pub struct ServerRepository {
    pub pool: PgPool,
}

impl ServerRepository {
    /// Creates a new `ServerRepository` instance.
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a new server owned by the specified user and automatically
    /// adds the owner to `server_members` with role `'owner'`.
    ///
    /// # Arguments
    /// * `user_id` - UUID of the server owner
    /// * `payload` - Server creation data
    pub async fn create(
        &self,
        user_id: Uuid,
        payload: &CreateServer,
    ) -> Result<Server, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let server = sqlx::query_as!(
            Server,
            r#"
            INSERT INTO servers (owner_id, name, description, is_public)
            VALUES ($1, $2, $3, $4)
            RETURNING
                server_id,
                owner_id,
                name,
                description,
                is_public,
                created_at,
                updated_at
            "#,
            user_id,
            payload.name,
            payload.description,
            payload.is_public.unwrap_or(true)
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO server_members (user_id, server_id, role)
            VALUES ($1, $2, 'owner')
            "#,
            user_id,
            server.server_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(server)
    }

    /// Retrieves a server by its ID.
    ///
    /// # Arguments
    /// * `server_id` - UUID of the server
    pub async fn find_by_id(&self, server_id: Uuid) -> Result<Option<Server>, sqlx::Error> {
        sqlx::query_as!(
            Server,
            r#"
            SELECT
                server_id,
                owner_id,
                name,
                description,
                is_public,
                created_at,
                updated_at
            FROM servers
            WHERE server_id = $1
            "#,
            server_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Retrieves all public servers.
    pub async fn list_public(&self) -> Result<Vec<Server>, sqlx::Error> {
        sqlx::query_as!(
            Server,
            r#"
            SELECT
                server_id,
                owner_id,
                name,
                description,
                is_public,
                created_at,
                updated_at
            FROM servers
            WHERE is_public = true
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Retrieves all servers a user is currently a member of.
    ///
    /// Joins `server_members` with `servers` to find every server
    /// the given user has joined, ordered by when they joined.
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user
    ///
    /// # Returns
    /// A vector of servers the user is a member of
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Server>, sqlx::Error> {
        sqlx::query_as!(
            Server,
            r#"
            SELECT s.server_id, s.owner_id, s.name, s.description, s.is_public, s.created_at, s.updated_at
            FROM servers s
            INNER JOIN server_members sm ON s.server_id = sm.server_id
            WHERE sm.user_id = $1
            ORDER BY sm.joined_at ASC
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Updates a server. Only the provided fields are updated.
    ///
    /// # Arguments
    /// * `server_id` - UUID of the server to update
    /// * `payload` - Server update data
    pub async fn update(
        &self,
        server_id: Uuid,
        payload: &UpdateServer,
    ) -> Result<Option<Server>, sqlx::Error> {
        sqlx::query_as!(
            Server,
            r#"
            UPDATE servers
            SET
                name        = COALESCE($1, name),
                description = COALESCE($2, description),
                is_public   = COALESCE($3, is_public)
            WHERE server_id = $4
            RETURNING server_id, owner_id, name, description, is_public, created_at, updated_at
            "#,
            payload.name,
            payload.description,
            payload.is_public,
            server_id,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a server by its ID.
    ///
    /// # Arguments
    /// * `server_id` - UUID of the server to delete
    ///
    /// Returns the number of rows affected.
    pub async fn delete(&self, server_id: Uuid) -> Result<u64, sqlx::Error> {
        sqlx::query!("DELETE FROM servers WHERE server_id = $1", server_id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
    }
}
