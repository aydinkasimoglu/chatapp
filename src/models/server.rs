use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::enums::MemberRole;

#[derive(Debug, Clone, FromRow)]
pub struct Server {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct ServerMember {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub nickname: Option<String>,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServer {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServer {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct JoinServer {
    pub nickname: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateMember {
    pub nickname: Option<String>,
    pub role: Option<MemberRole>,
}

#[derive(Debug, Serialize)]
pub struct ServerResponse {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Server> for ServerResponse {
    fn from(server: Server) -> Self {
        Self {
            server_id: server.server_id,
            owner_id: server.owner_id,
            name: server.name,
            description: server.description,
            is_public: server.is_public,
            created_at: server.created_at,
            updated_at: server.updated_at,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ServerMemberResponse {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub nickname: Option<String>,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

impl From<ServerMember> for ServerMemberResponse {
    fn from(member: ServerMember) -> Self {
        Self {
            user_id: member.user_id,
            server_id: member.server_id,
            nickname: member.nickname,
            role: member.role,
            joined_at: member.joined_at,
        }
    }
}
