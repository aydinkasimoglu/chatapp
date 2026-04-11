#![allow(dead_code)]

use std::{future::Future, pin::Pin};

use futures::FutureExt;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use chatapp::state::AppState;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const TEST_JWT_SECRET: &str = "01234567890123456789012345678901";
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/20260320095801_initial_schema.sql"),
    include_str!("../../migrations/20260324123000_add_friendships.sql"),
    include_str!("../../migrations/20260324123001_add_user_blocks.sql"),
    include_str!("../../migrations/20260324123002_add_user_presence.sql"),
    include_str!("../../migrations/20260324123003_add_refresh_tokens.sql"),
    include_str!("../../migrations/20260324123004_rename_messages_table.sql"),
    include_str!("../../migrations/20260324123005_add_dm_schema.sql"),
    include_str!("../../migrations/20260410110000_enforce_dm_group_member_counts.sql"),
];

/// Temporary migrated PostgreSQL schema used by tests.
pub struct TestDatabase {
    pub pool: PgPool,
    schema_name: String,
    database_url: String,
}

impl TestDatabase {
    /// Creates a fully migrated isolated schema for a test.
    pub async fn new() -> Result<Self, BoxError> {
        let database_url = std::env::var("DATABASE_URL")?;
        let schema_name = format!("test_{}", Uuid::new_v4().simple());

        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await?;
        let create_schema = format!(r#"CREATE SCHEMA "{}""#, schema_name);
        sqlx::query(&create_schema).execute(&admin_pool).await?;

        let schema_name_for_pool = schema_name.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _meta| {
                let schema_name = schema_name_for_pool.clone();
                Box::pin(async move {
                    let search_path = format!(r#"SET search_path TO "{}""#, schema_name);
                    sqlx::query(&search_path).execute(connection).await?;
                    Ok(())
                })
            })
            .connect(&database_url)
            .await?;

        for migration in MIGRATIONS {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }

        Ok(Self {
            pool,
            schema_name,
            database_url,
        })
    }

    /// Drops the temporary schema and closes the connection pool.
    pub async fn cleanup(self) -> Result<(), BoxError> {
        self.pool.close().await;

        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&self.database_url)
            .await?;
        let drop_schema = format!(r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#, self.schema_name);
        sqlx::query(&drop_schema).execute(&admin_pool).await?;

        Ok(())
    }
}

/// Runs an async test against an isolated migrated schema.
pub async fn run_db_test<F>(test: F)
where
    F: for<'a> FnOnce(&'a TestDatabase) -> Pin<Box<dyn Future<Output = Result<(), BoxError>> + Send + 'a>>,
{
    let database = TestDatabase::new()
        .await
        .unwrap_or_else(|error| panic!("failed to create test database: {error}"));
    let test_result = std::panic::AssertUnwindSafe(test(&database))
        .catch_unwind()
        .await;
    let cleanup_result = database.cleanup().await;

    if let Err(error) = cleanup_result {
        panic!("failed to clean up test database: {error}");
    }

    match test_result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("test failed: {error}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

/// Builds an application state backed by the isolated test database.
pub fn build_test_state(pool: PgPool) -> Result<AppState, BoxError> {
    AppState::new(pool, TEST_JWT_SECRET.to_string()).map_err(Into::into)
}

/// Inserts an active user row for tests and returns the generated user identifier.
pub async fn insert_user(pool: &PgPool, username: &str) -> Result<Uuid, sqlx::Error> {
    let email = format!("{}-{}@example.com", username, Uuid::new_v4().simple());

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (username, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING user_id
        "#,
    )
    .bind(username)
    .bind(email)
    .bind("test-password-hash")
    .fetch_one(pool)
    .await
}

/// Inserts a directional user block for tests.
pub async fn insert_block(
    pool: &PgPool,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO user_blocks (blocker_id, blocked_id)
        VALUES ($1, $2)
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(pool)
    .await?;

    Ok(())
}
