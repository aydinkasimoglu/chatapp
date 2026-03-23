use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::{CreateServer, Server, UpdateServer},
    repositories::server::ServerRepository,
};

#[derive(Clone)]
pub struct ServerService {
    pub repository: ServerRepository,
}

impl ServerService {
    pub fn new(repository: ServerRepository) -> Self {
        Self { repository }
    }

    /// Creates a new server owned by the specified user.
    ///
    /// # Validation
    /// - Server name must not be empty and must be at most 100 characters
    /// - Description (if provided) must be at most 500 characters
    ///
    /// # Errors
    /// Returns `ServiceError::Database` if the database operation fails.
    pub async fn create(
        &self,
        user_id: Uuid,
        payload: CreateServer,
    ) -> Result<Server, ServiceError> {

        self.validate_name(&payload.name)?;

        // Validate description if provided
        if let Some(ref desc) = payload.description {
            self.validate_description(desc)?;
        }

        let server = self.repository.create(user_id, &payload).await?;
        Ok(server)
    }

    /// Retrieves a server by its ID.
    ///
    /// # Errors
    /// Returns `ServiceError::NotFound` if the server doesn't exist.
    /// Returns `ServiceError::Database` for database errors.
    pub async fn find_by_id(&self, server_id: Uuid) -> Result<Server, ServiceError> {
        self.repository
            .find_by_id(server_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Retrieves all servers a user is currently a member of.
    ///
    /// Returns servers ordered by creation time (newest first).
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Server>, ServiceError> {
        self.repository.list_by_user(user_id).await.map_err(Into::into)
    }

    /// Lists all public servers.
    ///
    /// Returns servers ordered by creation time (newest first).
    pub async fn list_public(&self) -> Result<Vec<Server>, ServiceError> {
        self.repository.list_public().await.map_err(Into::into)
    }

    /// Updates a server.
    ///
    /// Only the provided fields in the payload are updated.
    ///
    /// # Validation
    /// - If updating name, it must not be empty and must be at most 100 characters
    /// - If updating description, it must be at most 500 characters
    ///
    /// # Errors
    /// Returns `ServiceError::NotFound` if the server doesn't exist.
    /// Returns `ServiceError::Database` for database errors.
    pub async fn update(
        &self,
        server_id: Uuid,
        payload: UpdateServer,
    ) -> Result<Server, ServiceError> {
        // Validate name if provided
        if let Some(ref name) = payload.name {
            self.validate_name(name)?;
        }

        // Validate description if provided
        if let Some(ref desc) = payload.description {
            self.validate_description(desc)?;
        }

        self.repository
            .update(server_id, &payload)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Deletes a server by its UUID.
    ///
    /// # Errors
    /// Returns `ServiceError::NotFound` if the server doesn't exist.
    /// Returns `ServiceError::Database` for database errors.
    pub async fn delete(&self, server_id: Uuid) -> Result<(), ServiceError> {
        if self.repository.delete(server_id).await? == 0 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }
    
    fn validate_name(&self, name: &str) -> Result<(), ServiceError> {
        if name.trim().is_empty() {
            return Err(ServiceError::ValidationError("Server name cannot be empty".to_string()));
        }
        if name.len() > 100 {
            return Err(ServiceError::ValidationError("Server name must not exceed 100 characters".to_string()));
        }
        Ok(())
    }

    fn validate_description(&self, desc: &str) -> Result<(), ServiceError> {
        if desc.len() > 500 {
            return Err(ServiceError::ValidationError("Description must not exceed 500 characters".to_string()));
        }
        Ok(())
    }
}
