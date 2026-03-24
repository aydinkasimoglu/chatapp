use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::BlockResponse,
    repositories::{
        blocks::BlockRepository, friendship::FriendshipRepository, user::UserRepository,
    },
};

/// Service for managing user-block operations.
///
/// Handles blocking and unblocking users. When a user is blocked, any existing
/// friendship or pending request between the two users is automatically removed,
/// leaving the social graph in a clean state.
#[derive(Clone)]
pub struct BlockService {
    repository: BlockRepository,
    friendship_repository: FriendshipRepository,
    user_repository: UserRepository,
}

impl BlockService {
    /// Creates a new `BlockService` instance.
    ///
    /// # Arguments
    /// * `repository` - Block repository for `user_blocks` table operations
    /// * `friendship_repository` - Used to clean up existing relationships when blocking
    /// * `user_repository` - Used to verify the target user exists and is active
    pub fn new(
        repository: BlockRepository,
        friendship_repository: FriendshipRepository,
        user_repository: UserRepository,
    ) -> Self {
        Self {
            repository,
            friendship_repository,
            user_repository,
        }
    }

    /// Blocks `target_id` from the perspective of `blocker_id`.
    ///
    /// Any pending request or accepted friendship between the two users is silently
    /// deleted before the block is created. Returns the new block record on success.
    ///
    /// # Errors
    /// - `ValidationError` if `blocker_id == target_id`.
    /// - `NotFound` if the target user does not exist or is deactivated.
    /// - `AlreadyBlocked` if a block from `blocker_id` → `target_id` already exists.
    pub async fn block_user(
        &self,
        blocker_id: Uuid,
        target_id: Uuid,
    ) -> Result<BlockResponse, ServiceError> {
        if blocker_id == target_id {
            return Err(ServiceError::ValidationError(
                "You cannot block yourself".to_string(),
            ));
        }

        let user = self
            .user_repository
            .find_active_by_id(target_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        // Remove any existing friendship or pending request between the two users
        // before creating the block so the social graph stays consistent.
        self.friendship_repository
            .delete_between(blocker_id, target_id)
            .await?;

        let block = self
            .repository
            .create(blocker_id, target_id)
            .await
            .map_err(|err| {
                if let Some(db_err) = err.as_database_error() {
                    if let Some(constraint) = db_err.constraint() {
                        if constraint == "uq_ub_pair" {
                            return ServiceError::AlreadyBlocked;
                        }
                    }
                }
                ServiceError::Database(err)
            })?;

        Ok(BlockResponse {
            block_id: block.block_id,
            blocked_user_id: user.user_id,
            blocked_username: user.username,
            blocked_email: user.email,
            created_at: block.created_at,
        })
    }

    /// Removes the block that `blocker_id` placed on `target_id`.
    ///
    /// After unblocking, neither user has any relationship record with the other.
    /// Either may send a new friend request at any time.
    ///
    /// # Errors
    /// - `NotFound` if no active block from `blocker_id` → `target_id` exists.
    pub async fn unblock_user(
        &self,
        blocker_id: Uuid,
        target_id: Uuid,
    ) -> Result<(), ServiceError> {
        let deleted = self.repository.delete(blocker_id, target_id).await?;
        if deleted {
            Ok(())
        } else {
            Err(ServiceError::NotFound)
        }
    }

    /// Returns all users that `user_id` has currently blocked.
    ///
    /// Deactivated accounts are excluded from the results.
    pub async fn list_blocked(&self, user_id: Uuid) -> Result<Vec<BlockResponse>, ServiceError> {
        Ok(self
            .repository
            .list_blocked_by(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
