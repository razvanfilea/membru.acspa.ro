use askama::Template;
use axum::response::{IntoResponse, Response};

#[derive(Template)]
#[template(path = "components/error_bubble.html")]
pub struct ErrorBubbleTemplate<'a> {
    pub message: &'a str,
}

pub fn error_bubble_response(message: impl AsRef<str>) -> Response {
    ErrorBubbleTemplate {
        message: message.as_ref(),
    }
    .into_response()
}
