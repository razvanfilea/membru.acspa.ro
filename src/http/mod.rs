use crate::http::auth::UserAuthenticator;
use crate::model::location::Location;
use crate::utils::local_time;
use anyhow::Context;
use axum::Router;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::AuthManagerLayerBuilder;
use sqlx::{query, query_as, SqlitePool};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::time::interval;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{info, Level};

mod auth;
mod pages;

#[derive(Clone)]
pub struct AppState {
    pub read_pool: SqlitePool,
    pub write_pool: SqlitePool,
    pub location: Arc<Location>,
    pub reservation_notifier: Arc<watch::Sender<()>>,
}

impl AppState {
    pub async fn new(read_pool: SqlitePool, write_pool: SqlitePool) -> Self {
        let (tx, _) = watch::channel(());
        let location = query_as!(Location, "select * from locations")
            .fetch_one(&read_pool)
            .await
            .expect("No locations found");

        Self {
            location: Arc::new(location),
            read_pool,
            write_pool,
            reservation_notifier: Arc::new(tx),
        }
    }
}

pub async fn periodic_cleanup_of_waiting_reservations(state: AppState) {
    let notifier = state.reservation_notifier;
    let pool = state.write_pool;
    let mut interval = interval(std::time::Duration::from_secs(10 * 60));

    loop {
        interval.tick().await;

        let current_time = local_time();
        let current_date = current_time.date();
        let current_hour = current_time.hour();

        let rows_affected = query!(
            "delete from reservations where in_waiting = true and (date < $1 or (date == $1 and hour <= $2))",
                current_date,
                current_hour)
            .execute(&pool)
            .await
            .expect("Database error")
            .rows_affected();

        if rows_affected != 0
        {
            info!("Deleted {rows_affected} expired reservations");
        }

        let _ = notifier.send(());
    }
}

pub async fn http_server(app_state: AppState, session_store: SqliteStore) -> anyhow::Result<()> {
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(60)));

    let auth_layer = AuthManagerLayerBuilder::new(
        UserAuthenticator::new(app_state.read_pool.clone()),
        session_layer,
    )
    .build();

    let app = Router::new()
        .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
        .merge(pages::router())
        .with_state(app_state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(auth_layer);

    let port_str = std::env::var("SERVER_PORT").context("Failed to get server port")?;
    let port: u16 = port_str.parse().context("Invalid port")?;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let http_service = app.into_make_service();

    println!("Server started on port {port}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, http_service)
        .await
        .context("Failed to start server")
}
