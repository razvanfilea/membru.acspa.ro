use crate::http::pages::notification_template::error_bubble_response;
use axum::response::{IntoResponse, Response};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Axum error: `{0}`")]
    Response(#[from] axum::http::Error),
    #[error("Database error: `{0}`")]
    Database(#[from] sqlx::Error),
    #[error("Failed to generate HTML: `{0}`")]
    Template(#[from] askama::Error),
    #[error("{0}")]
    Text(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let message = self.to_string();
        error!("{message}");
        error_bubble_response(message)
    }
}

pub type HttpResult<T = Response> = Result<T, HttpError>;
