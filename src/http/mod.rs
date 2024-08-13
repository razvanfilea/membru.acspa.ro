use crate::http::auth::UserAuthenticator;
use crate::model::location::Location;
use axum::Router;
use axum_login::tower_sessions::cookie::time::Duration;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::AuthManagerLayerBuilder;
use sqlx::{query_as, SqlitePool};
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Context;
use tokio::sync::watch;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;

mod auth;
mod error;
mod pages;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub location: Location,
    pub reservation_notifier: Arc<watch::Sender<()>>,
}

impl AppState {
    pub async fn new(pool: SqlitePool) -> Self {
        let (tx, _) = watch::channel(());

        Self {
            location: query_as!(Location, "select * from locations")
                .fetch_one(&pool)
                .await
                .expect("No locations found"),
            pool,
            reservation_notifier: Arc::new(tx)
        }
    }
}

pub async fn http_server(app_state: AppState, session_store: SqliteStore) -> anyhow::Result<()> {
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(90)))
        .with_secure(false);

    let auth_layer = AuthManagerLayerBuilder::new(
        UserAuthenticator::new(app_state.pool.clone()),
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
    axum::serve(listener, http_service).await.context("Failed to start server")
}
