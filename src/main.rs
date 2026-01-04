use axum_login::tower_sessions::ExpiredDeletion;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;
use tokio::task;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::http::{AppState, http_server, periodic_cleanup_of_waiting_reservations};

mod http;
mod model;
mod reservation;
mod utils;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "info,axum_login=off,tower_sessions=off,sqlx=warn,tower_http=info".into(),
        )))
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("Failed to get database URL");
    let connection_options = SqliteConnectOptions::from_str(&database_url)
        .expect("Failed to parse Database URL")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
        .pragma("temp_store", "memory")
        .pragma("cache_size", "-20000");

    let read_pool = SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_with(connection_options.clone().read_only(true))
        .await
        .expect("Failed to create Read-Only DB Pool");

    let write_pool = SqlitePoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_with(connection_options.optimize_on_close(true, None))
        .await
        .expect("Failed to create Write DB Pool");

    sqlx::migrate!()
        .run(&write_pool)
        .await
        .expect("Failed to run DB migrations");

    let session_store = SqliteStore::new(write_pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to run schema migration for authentication");
    if let Err(e) = session_store.delete_expired().await {
        warn!("Failed to clean up expired sessions: {e}");
    }

    let app_state = AppState::new(read_pool, write_pool).await;

    task::spawn(periodic_cleanup_of_waiting_reservations(app_state.clone()));

    http_server(app_state, session_store).await;

    Ok(())
}
