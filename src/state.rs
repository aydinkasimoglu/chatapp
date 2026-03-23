use axum::extract::FromRef;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, broadcast};

use crate::services::{
    auth::AuthService, server::ServerService, user::UserService,
};

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub user_service: UserService,
    pub server_service: ServerService,
    pub rooms: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
}

impl FromRef<AppState> for AuthService {
    fn from_ref(input: &AppState) -> Self {
        input.auth_service.clone()
    }
}
