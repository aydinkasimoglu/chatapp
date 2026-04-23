use serde::Serialize;
use uuid::Uuid;

/// Cursor-based API response for paginated DM message lists.
#[derive(Debug, Serialize)]
pub struct CursorPaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub limit: i64,
    pub before_message_id: Option<Uuid>,
    pub next_before_message_id: Option<Uuid>,
    pub has_older: bool,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
