use axum_login::tower_sessions::ExpiredDeletion;
use tokio::task;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::http::{AppState, http_server, periodic_cleanup_of_waiting_reservations};

mod db;
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
    let (read_pool, write_pool) = db::init_pools(&database_url).await;

    db::run_migrations(&write_pool).await;

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
