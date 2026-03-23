use crate::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    models::{CreateServer, ServerResponse, UpdateServer},
    state::AppState,
};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

pub async fn create_server_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Json(payload): Json<CreateServer>,
) -> Result<(StatusCode, Json<ServerResponse>), ServiceError> {
    let server = state.server_service.create(user_id, payload).await?;
    Ok((StatusCode::CREATED, Json(server.into())))
}

pub async fn get_server_handler(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerResponse>, ServiceError> {
    let server = state.server_service.find_by_id(server_id).await?;
    Ok(Json(server.into()))
}

pub async fn get_public_servers_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServerResponse>>, ServiceError> {
    let servers = state.server_service.list_public().await?;
    Ok(Json(
        servers.into_iter().map(ServerResponse::from).collect(),
    ))
}

pub async fn get_my_servers_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
) -> Result<Json<Vec<ServerResponse>>, ServiceError> {
    let servers = state.server_service.list_by_user(user_id).await?;
    Ok(Json(
        servers.into_iter().map(ServerResponse::from).collect(),
    ))
}

pub async fn update_server_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<UpdateServer>,
) -> Result<Json<ServerResponse>, ServiceError> {
    let server = state.server_service.find_by_id(server_id).await?;
    if server.owner_id != user_id {
        return Err(ServiceError::Unauthorized);
    }
    let updated = state.server_service.update(server_id, payload).await?;
    Ok(Json(updated.into()))
}

pub async fn delete_server_handler(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, ServiceError> {
    let server = state.server_service.find_by_id(server_id).await?;
    if server.owner_id != user_id {
        return Err(ServiceError::Unauthorized);
    }
    state.server_service.delete(server_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

