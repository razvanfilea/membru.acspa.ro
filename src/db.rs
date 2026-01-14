use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;
use std::time::Duration;

/// Initializes the Read and Write database pools with specific SQLite optimizations.
pub async fn init_pools(database_url: &str) -> (SqlitePool, SqlitePool) {
    let connection_options = SqliteConnectOptions::from_str(database_url)
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

    (read_pool, write_pool)
}

/// Runs standard database migrations.
pub async fn run_migrations(pool: &SqlitePool) {
    sqlx::migrate!()
        .run(pool)
        .await
        .expect("Failed to run DB migrations");
}
