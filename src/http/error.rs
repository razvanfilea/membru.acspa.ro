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
    Message(String),
    #[error("User not logged in")]
    Unauthorized,
}

pub trait OrBail<T> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError>;
}

// Implementation for Option
impl<T> OrBail<T> for Option<T> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::Message(msg.into()))
    }
}

// Implementation for Result
impl<T, E> OrBail<T> for Result<T, E> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError> {
        self.map_err(|_| HttpError::Message(msg.into()))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let message = self.to_string();
        error!("{message}");
        error_bubble_response(message)
    }
}

pub type HttpResult<T = Response> = Result<T, HttpError>;
