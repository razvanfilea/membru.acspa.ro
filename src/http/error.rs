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

pub fn bail(msg: impl Into<String>) -> HttpError {
    HttpError::Message(msg.into())
}

pub trait OrBail<T> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError>;
}

pub trait OrBailWith<T, E> {
    fn or_bail_with<S: ToString>(self, f: impl FnOnce(E) -> S) -> Result<T, HttpError>;
}

impl<T> OrBail<T> for Option<T> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::Message(msg.into()))
    }
}

impl<T, E> OrBail<T> for Result<T, E> {
    fn or_bail(self, msg: impl Into<String>) -> Result<T, HttpError> {
        self.map_err(|_| HttpError::Message(msg.into()))
    }
}

impl<T, E> OrBailWith<T, E> for Result<T, E> {
    fn or_bail_with<S: ToString>(self, f: impl FnOnce(E) -> S) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::Message(f(e).to_string()))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        error!("{self}");
        let message = match self {
            // A missing id, e.g. an old link to a deleted member
            HttpError::Database(sqlx::Error::RowNotFound) => "Elementul căutat nu există".to_string(),
            error => error.to_string(),
        };
        error_bubble_response(message)
    }
}

pub type HttpResult<T = Response> = Result<T, HttpError>;
