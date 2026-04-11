use axum::extract::FromRef;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{
    repositories::{
        blocks::BlockRepository, dm::DmRepository, friendship::FriendshipRepository,
        presence::PresenceRepository, refresh_token::RefreshTokenRepository,
        server::ServerRepository, user::UserRepository,
    },
    services::{
        auth::{AuthService, AuthServiceInitError},
        blocks::BlockService,
        dm::DmService,
        friendship::FriendshipService,
        presence::PresenceService,
        server::ServerService,
        user::UserService,
    },
};

/// Initialization errors for `AppState`.
#[derive(Debug, thiserror::Error)]
pub enum AppStateInitError {
    #[error(transparent)]
    Auth(#[from] AuthServiceInitError),
}

#[derive(Clone)]
pub struct AppState {
    pub auth_service:       AuthService,
    pub user_service:       UserService,
    pub server_service:     ServerService,
    pub friendship_service: FriendshipService,
    pub block_service:      BlockService,
    pub dm_service:         DmService,
    pub presence_service:   PresenceService,
    pub rooms: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub presence_tx: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_secret: String) -> Result<Self, AppStateInitError> {
        let rooms = Arc::new(Mutex::new(HashMap::new()));
        let presence_tx = Arc::new(Mutex::new(HashMap::new()));
        let user_repository = UserRepository::new(pool.clone());
        let server_repository = ServerRepository::new(pool.clone());
        let friendship_repository = FriendshipRepository::new(pool.clone());
        let presence_repository = PresenceRepository::new(pool.clone());
        let block_repository = BlockRepository::new(pool.clone());
        let dm_repository = DmRepository::new(pool.clone());
        let refresh_token_repository = RefreshTokenRepository::new(pool);

        Ok(Self {
            auth_service: AuthService::new(
                user_repository.clone(),
                refresh_token_repository,
                jwt_secret,
            )?,
            user_service: UserService::new(user_repository.clone()),
            server_service: ServerService::new(server_repository),
            friendship_service: FriendshipService::new(
                friendship_repository.clone(),
                user_repository.clone(),
                block_repository.clone(),
            ),
            block_service: BlockService::new(
                block_repository.clone(),
                friendship_repository,
                user_repository.clone(),
            ),
            dm_service: DmService::new(
                dm_repository,
                user_repository,
                block_repository.clone(),
                presence_tx.clone(),
            ),
            presence_service: PresenceService::new(presence_repository),
            rooms,
            presence_tx,
        })
    }
}

impl FromRef<AppState> for AuthService {
    fn from_ref(input: &AppState) -> Self {
        input.auth_service.clone()
    }
}
