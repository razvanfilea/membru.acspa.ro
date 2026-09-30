use crate::http::auth::UserAuthenticator;
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::location::Location;
use crate::reservation::{HISTORY_RETENTION_DAYS, delete_expired_waiting};
use crate::utils::local_time;
use askama::Template;
use axum::Router;
use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum_login::AuthManagerLayerBuilder;
use axum_login::tower_sessions::cookie::SameSite;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use sqlx::{SqlitePool, query, query_as};
use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal;
use tokio::sync::watch;
use tokio::time::interval;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{Level, debug, error, info};

mod auth;
mod error;
mod pages;
mod response;
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

    const CSS_VERSION: &str = env!("CSS_VERSION");
}

/// Deletes expired waiting-list reservations and reservation history past its retention
pub async fn periodic_cleanup(state: AppState) {
    const CLEANUP_INTERVAL: Duration = Duration::from_mins(30);

    let notifier = state.reservation_notifier;
    let pool = state.write_pool;
    let mut interval = interval(CLEANUP_INTERVAL);

    loop {
        interval.tick().await;

        let current_time = local_time();
        let current_date = current_time.date();
        let current_hour = current_time.hour();

        match delete_expired_waiting(&pool, current_date, current_hour).await {
            Ok(rows_affected) => {
                if rows_affected != 0 {
                    info!("Deleted {rows_affected} expired reservations");
                    let _ = notifier.send(());
                }
            }
            Err(e) => {
                error!("Failed to delete expired reservations: {e}");
            }
        }

        let history_result = query!(
            "delete from reservation_history where changed_at < datetime('now', '-' || $1 || ' days')",
            HISTORY_RETENTION_DAYS
        )
        .execute(&pool)
        .await;

        if let Err(e) = history_result {
            error!("Failed to delete old reservation history: {e}");
        }
    }
}

async fn handler_404() -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "404.html")]
    struct NotFoundTemplate;

    (StatusCode::NOT_FOUND, NotFoundTemplate.try_into_response())
}

/// Logs requests that change data or fail at info level, everything else at debug level
async fn log_response(request: Request, next: Next) -> Response {
    let is_read_only = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let start = Instant::now();

    let response = next.run(request).await;

    let latency_ms = start.elapsed().as_millis();
    let status = response.status().as_u16();

    if !is_read_only || status >= 400 {
        info!(status, latency_ms, "finished processing request");
    } else {
        debug!(status, latency_ms, "finished processing request");
    }

    response
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

pub async fn http_server(app_state: AppState, session_store: SqliteStore) {
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
        .merge(pages::router())
        .with_state(app_state)
        .fallback(handler_404)
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer((
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(()),
            middleware::from_fn(log_response),
            TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(10),
            ),
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
