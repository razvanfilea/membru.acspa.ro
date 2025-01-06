use askama::Template;
use axum::response::{IntoResponse, Response};
use template_response::TemplateResponse;

#[derive(Template, TemplateResponse)]
#[template(path = "components/bubble/error.html")]
struct ErrorBubbleTemplate<'a> {
    pub message: &'a str,
}

pub fn error_bubble_response(message: impl AsRef<str>) -> Response {
    ErrorBubbleTemplate {
        message: message.as_ref(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "components/bubble/notification.html")]
pub struct NotificationBubbleResponse<'a> {
    pub message: &'a str,
}
