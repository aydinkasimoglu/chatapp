//! Data access layer for database operations.
//!
//! Contains repositories that provide type-safe database access
//! using SQLx with PostgreSQL.

pub mod blocks;
pub mod friendship;
pub mod presence;
pub mod refresh_token;
pub mod server;
pub mod user;
