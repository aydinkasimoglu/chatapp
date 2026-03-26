use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::{OnlineFriendResponse, PresenceStatus},
    repositories::presence::PresenceRepository,
};

#[derive(Clone)]
pub struct PresenceService {
    repository: PresenceRepository,
}

impl PresenceService {
    pub fn new(repository: PresenceRepository) -> Self {
        Self { repository }
    }

    /// Registers a new WebSocket session for `user_id`.
    /// Returns the DB-generated session UUID to be held for the lifetime of the connection.
    pub async fn connect(&self, user_id: Uuid) -> Result<Uuid, ServiceError> {
        Ok(self.repository.connect(user_id).await?)
    }

    /// Updates the heartbeat for `session_id`.
    ///
    /// Accepts "idle" as the only alternative; anything else is treated as Online.
    pub async fn heartbeat(&self, session_id: Uuid, status: &str) -> Result<(), ServiceError> {
        let status = if status == "idle" {
            PresenceStatus::Idle
        } else {
            PresenceStatus::Online
        };
        self.repository.heartbeat(session_id, status).await?;
        Ok(())
    }

    /// Removes the session on disconnect.
    pub async fn disconnect(&self, session_id: Uuid) -> Result<(), ServiceError> {
        self.repository.disconnect(session_id).await?;
        Ok(())
    }

    /// Evicts stale sessions (heartbeat older than 60s). Call from a background task.
    pub async fn cleanup_stale(&self) -> Result<u64, ServiceError> {
        Ok(self.repository.cleanup_stale().await?)
    }

    /// Returns the online/idle friends of `user_id`.
    pub async fn online_friends(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OnlineFriendResponse>, ServiceError> {
        let records = self.repository.online_friends(user_id).await?;
        Ok(records.into_iter().map(OnlineFriendResponse::from).collect())
    }
}
