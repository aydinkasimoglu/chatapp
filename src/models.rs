use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

// =============================================================
// CORE DATABASE MODELS
// =============================================================

/// Represents a full user row as returned from the database.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id:       Uuid,
    pub username:      String,
    pub email:         String,
    pub password_hash: String,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Server {
    pub server_id:   Uuid,
    pub owner_id:    Uuid,
    pub name:        String,
    pub description: Option<String>,  // nullable in DB
    pub is_public:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ServerMember {
    pub user_id:   Uuid,
    pub server_id: Uuid,
    pub nickname:  Option<String>,    // nullable in DB
    pub role:      MemberRole,
    pub joined_at: DateTime<Utc>,
}

// =============================================================
// REQUEST MODELS (incoming API payloads)
// =============================================================

/// Payload for creating a new user.
///
/// The `password` field is plain-text here — the repository
/// is responsible for hashing it before persisting.
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email:    String,
    pub password: String,
}

/// Payload for partially updating a user's profile.
///
/// All fields are optional — only `Some(value)` fields are applied.
/// Password changes must go through `UpdatePassword` instead.
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email:    Option<String>,
}

/// Payload for changing a user's password.
///
/// Requires the current password for verification before
/// the repository accepts the new one.
#[derive(Debug, Deserialize)]
pub struct UpdatePassword {
    pub current_password: String,
    pub new_password:     String,
}

#[derive(Debug, Deserialize)]
pub struct CreateServer {
    pub name:        String,
    pub description: Option<String>,
    pub is_public:   Option<bool>,    // defaults to TRUE in DB
}

#[derive(Debug, Deserialize)]
pub struct UpdateServer {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub is_public:   Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct JoinServer {
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMember {
    pub nickname: Option<String>,
    pub role:     Option<MemberRole>,
}

/// Request payload for user authentication.
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

// =============================================================
// RESPONSE MODELS (outgoing API shapes)
// =============================================================

/// Safe public representation of a user for API responses.
///
/// Intentionally omits `password_hash` and `is_active`.
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id:    Uuid,
    pub username:   String,
    pub email:      String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            user_id:    user.user_id,
            username:   user.username,
            email:      user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerResponse {
    pub server_id:   Uuid,
    pub owner_id:    Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub is_public:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

impl From<Server> for ServerResponse {
    fn from(server: Server) -> Self {
        Self {
            server_id:   server.server_id,
            owner_id:    server.owner_id,
            name:        server.name,
            description: server.description,
            is_public:   server.is_public,
            created_at:  server.created_at,
            updated_at:  server.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerMemberResponse {
    pub user_id:   Uuid,
    pub server_id: Uuid,
    pub nickname:  Option<String>,
    pub role:      MemberRole,
    pub joined_at: DateTime<Utc>,
}

impl From<ServerMember> for ServerMemberResponse {
    fn from(member: ServerMember) -> Self {
        Self {
            user_id:   member.user_id,
            server_id: member.server_id,
            nickname:  member.nickname,
            role:      member.role,
            joined_at: member.joined_at,
        }
    }
}

/// Response containing a JWT token for authenticated requests.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

/// Standard error response format.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// JWT claims contained within a token.
/// 
/// `sub` is the subject (user ID) and `exp` is the expiration time as a Unix timestamp.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (User ID)
    pub exp: usize,  // Expiration time
}

// =============================================================
// ENUM
// =============================================================

/// Maps to the `member_role` PostgreSQL ENUM type.
///
/// The `sqlx::Type` derive handles encoding/decoding automatically.
/// The `rename_all` attribute matches the lowercase DB enum values.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Admin,
    Moderator,
    Member,
}