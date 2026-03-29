use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::{FriendResponse, FriendshipResponse, FriendshipStatus, PendingFriendRequestResponse},
    repositories::{
        blocks::BlockRepository, friendship::FriendshipRepository, user::UserRepository,
    },
};

/// Service for friendship and friend-request workflows.
///
/// Owns the business rules for sending requests, accepting/rejecting, canceling,
/// and removing friendships.
#[derive(Clone)]
pub struct FriendshipService {
    repository: FriendshipRepository,
    user_repository: UserRepository,
    block_repository: BlockRepository,
}

impl FriendshipService {
    /// Creates a new `FriendshipService` instance.
    pub fn new(
        repository: FriendshipRepository,
        user_repository: UserRepository,
        block_repository: BlockRepository,
    ) -> Self {
        Self {
            repository,
            user_repository,
            block_repository,
        }
    }

    /// Sends a new friend request from `requester_id` to the user with the given username.
    ///
    /// If the pair previously had a rejected request, that row is reopened as
    /// a new pending request.
    pub async fn send_request(
        &self,
        requester_id: Uuid,
        addressee_username: &str,
    ) -> Result<FriendshipResponse, ServiceError> {
        let addressee = self
            .user_repository
            .find_active_by_username(addressee_username)
            .await?
            .ok_or(ServiceError::NotFound)?;

        let addressee_id = addressee.user_id;

        if requester_id == addressee_id {
            return Err(ServiceError::ValidationError(
                "You cannot send a friend request to yourself".to_string(),
            ));
        }

        if self
            .block_repository
            .exists_between(requester_id, addressee_id)
            .await?
        {
            return Err(ServiceError::BlockedRelationship);
        }

        let friendship = match self
            .repository
            .find_between(requester_id, addressee_id)
            .await?
        {
            None => {
                self.repository
                    .create_request(requester_id, addressee_id)
                    .await?
            }
            Some(existing) if existing.status == FriendshipStatus::Accepted => {
                return Err(ServiceError::AlreadyFriends);
            }
            Some(existing) if existing.status == FriendshipStatus::Pending => {
                return Err(ServiceError::FriendRequestAlreadyPending);
            }
            Some(existing) => {
                self.repository
                    .reopen_request(existing.friendship_id, requester_id, addressee_id)
                    .await?
            }
        };

        Ok(friendship.into())
    }

    /// Lists accepted friends for a user.
    pub async fn list_friends(&self, user_id: Uuid) -> Result<Vec<FriendResponse>, ServiceError> {
        Ok(self
            .repository
            .list_friends(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Lists incoming pending requests for a user.
    pub async fn list_incoming_pending(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PendingFriendRequestResponse>, ServiceError> {
        Ok(self
            .repository
            .list_incoming_pending(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Lists outgoing pending requests for a user.
    pub async fn list_outgoing_pending(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PendingFriendRequestResponse>, ServiceError> {
        Ok(self
            .repository
            .list_outgoing_pending(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Accepts a pending friend request.
    ///
    /// Only the addressee can accept.
    pub async fn accept_request(
        &self,
        user_id: Uuid,
        friendship_id: Uuid,
    ) -> Result<FriendshipResponse, ServiceError> {
        let friendship = self
            .repository
            .find_by_id(friendship_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        if friendship.addressee_id != user_id {
            return Err(ServiceError::Unauthorized);
        }

        if friendship.status != FriendshipStatus::Pending {
            return Err(ServiceError::InvalidFriendRequestState);
        }

        let updated = self
            .repository
            .update_status(friendship_id, "accepted")
            .await?
            .ok_or(ServiceError::InvalidFriendRequestState)?;

        Ok(updated.into())
    }

    /// Rejects a pending friend request.
    ///
    /// Only the addressee can reject.
    pub async fn reject_request(
        &self,
        user_id: Uuid,
        friendship_id: Uuid,
    ) -> Result<FriendshipResponse, ServiceError> {
        let friendship = self
            .repository
            .find_by_id(friendship_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        if friendship.addressee_id != user_id {
            return Err(ServiceError::Unauthorized);
        }

        if friendship.status != FriendshipStatus::Pending {
            return Err(ServiceError::InvalidFriendRequestState);
        }

        let updated = self
            .repository
            .update_status(friendship_id, "rejected")
            .await?
            .ok_or(ServiceError::InvalidFriendRequestState)?;

        Ok(updated.into())
    }

    /// Cancels an outgoing pending friend request.
    ///
    /// Only the requester who created the pending request can cancel it.
    pub async fn cancel_request(
        &self,
        user_id: Uuid,
        friendship_id: Uuid,
    ) -> Result<(), ServiceError> {
        let friendship = self
            .repository
            .find_by_id(friendship_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        if friendship.requester_id != user_id {
            return Err(ServiceError::Unauthorized);
        }

        if friendship.status != FriendshipStatus::Pending {
            return Err(ServiceError::InvalidFriendRequestState);
        }

        let deleted = self
            .repository
            .delete_pending_request(friendship_id, user_id)
            .await?;

        if deleted {
            Ok(())
        } else {
            Err(ServiceError::InvalidFriendRequestState)
        }
    }

    /// Removes an accepted friendship.
    ///
    /// Either participant may remove the friendship.
    pub async fn remove_friend(
        &self,
        user_id: Uuid,
        friendship_id: Uuid,
    ) -> Result<(), ServiceError> {
        let friendship = self
            .repository
            .find_by_id(friendship_id)
            .await?
            .ok_or(ServiceError::NotFound)?;

        if friendship.requester_id != user_id && friendship.addressee_id != user_id {
            return Err(ServiceError::Unauthorized);
        }

        if friendship.status != FriendshipStatus::Accepted {
            return Err(ServiceError::InvalidFriendRequestState);
        }

        let deleted = self
            .repository
            .delete_accepted_friendship(friendship_id, user_id)
            .await?;

        if deleted {
            Ok(())
        } else {
            Err(ServiceError::InvalidFriendRequestState)
        }
    }
}
