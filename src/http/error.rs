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
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader {
    //     expected: String,
    //     found: String,
    // },
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            HttpError::Response(e) => error_bubble_response(e.to_string()),
            HttpError::Database(e) => {
                let message = e.to_string();
                error!("{message}");
                error_bubble_response(message)
            }
            HttpError::Template(e) => {
                let message = e.to_string();
                error!("{message}");
                error_bubble_response(message)
            } // HttpError::InvalidHeader { .. } => {}
        }
    }
}

pub type HttpResult<T = Response> = Result<T, HttpError>;
