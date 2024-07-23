use anyhow::Context;
use axum_login::tower_sessions::ExpiredDeletion;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::http::{http_server, AppState};

mod http;
mod model;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "info,axum_login=off,tower_sessions=off,sqlx=warn,tower_http=info".into(),
        )))
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("Failed to get database URL")?;
    let connection_options = SqliteConnectOptions::from_str(&database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
        .pragma("temp_store", "memory")
        .pragma("cache_size", "-20000");

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(connection_options)
        .await?;

    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to run schema migration for authentication");
    session_store.delete_expired().await?;

    let app_state = AppState::new(pool).await;

    http_server(app_state, session_store)
        .await
        .context("Failed to start HTTP Server")
}
