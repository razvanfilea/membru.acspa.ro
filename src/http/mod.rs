use crate::http::auth::UserAuthenticator;
use axum::Router;
use axum_login::tower_sessions::cookie::time::Duration;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::AuthManagerLayerBuilder;
use sqlx::SqlitePool;
use std::net::SocketAddr;
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
}

pub async fn http_server(app_state: AppState, session_store: SqliteStore) -> std::io::Result<()> {
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(90)));

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

    const PORT: u16 = 8080;

    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let http_service = app.into_make_service();

    println!("Server started on port {PORT}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, http_service).await
}
