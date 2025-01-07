use crate::http::auth::UserAuthenticator;
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::location::Location;
use crate::utils::local_time;
use askama::Template;
use axum::response::{IntoResponse, Response};
use axum::Router;
use axum_login::tower_sessions::cookie::SameSite;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::AuthManagerLayerBuilder;
use sqlx::{query, query_as, SqlitePool};
use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch;
use tokio::time::interval;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{error, info, Level};

mod auth;
mod error;
mod pages;
mod template_into_response;

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

        let query_result = query!(
            "delete from reservations where in_waiting = true and (date < $1 or (date == $1 and hour <= $2))",
                current_date,
                current_hour)
            .execute(&pool)
            .await;

        match query_result {
            Ok(result) => {
                let rows_affected = result.rows_affected();
                if rows_affected != 0 {
                    info!("Deleted {rows_affected} expired reservations");
                    let _ = notifier.send(());
                }
            }
            Err(e) => {
                error!("Failed to delete expired reservations: {e}");
            }
        }
    }
}

async fn handler_404() -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/404.html")]
    struct NotFoundTemplate;

    NotFoundTemplate.try_into_response()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic message".to_string()
    };

    error_bubble_response(details)
}

pub async fn http_server(app_state: AppState, session_store: SqliteStore, timetable_path: String) {
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(60)))
        .with_same_site(SameSite::Lax);

    let auth_layer = AuthManagerLayerBuilder::new(
        UserAuthenticator::new(app_state.read_pool.clone()),
        session_layer,
    )
    .build();

    let app = Router::new()
        .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
        .nest_service("/orar", tower_http::services::ServeDir::new(timetable_path))
        .merge(pages::router())
        .with_state(app_state)
        .fallback(handler_404)
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer((
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
            TimeoutLayer::new(std::time::Duration::from_secs(10)),
        ))
        .layer(auth_layer);

    let port_str = std::env::var("SERVER_PORT").expect("Failed to get server port");
    let port: u16 = port_str.parse().expect("Invalid port");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let http_service = app.into_make_service();

    println!("Server started on port {port}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to create TcpListener");
    axum::serve(listener, http_service)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to start HTTP server")
}
