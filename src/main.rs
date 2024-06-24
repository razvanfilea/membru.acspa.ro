use anyhow::Context;
use axum_login::tower_sessions::ExpiredDeletion;
use sqlx::sqlite::SqlitePoolOptions;
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::http::{http_server, AppState};

mod http;
mod model;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(EnvFilter::from_default_env())
        .init();

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect(&std::env::var("DATABASE_URL").context("Failed to get database URL")?)
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
