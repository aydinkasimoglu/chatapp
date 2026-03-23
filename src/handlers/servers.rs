use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{CreateServer, ServerResponse},
    state::AppState,
};

use axum::{Json, extract::State, http::StatusCode};

pub async fn create_server_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateServer>,
) -> Result<(StatusCode, Json<ServerResponse>), ServiceError> {
    let server = state.server_service.create(user_id, payload).await?;
    Ok((StatusCode::CREATED, Json(server.into())))
}

pub async fn get_public_servers_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServerResponse>>, ServiceError> {
    let servers = state.server_service.list_public().await?;
    Ok(Json(
        servers.into_iter().map(ServerResponse::from).collect(),
    ))
}
