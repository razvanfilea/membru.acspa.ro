use crate::http::template_into_response::TemplateIntoResponse;
use askama::Template;
use axum::response::{IntoResponse, Response};

pub fn error_bubble_response(message: impl AsRef<str>) -> Response {
    #[derive(Template)]
    #[template(path = "components/bubble/error.html")]
    struct ErrorBubbleTemplate<'a> {
        message: &'a str,
    }

    let bubble = ErrorBubbleTemplate {
        message: message.as_ref(),
    };
    let bubble = TemplateIntoResponse::into_response(bubble);
    ([("HX-Reswap", "none")], bubble).into_response()
}

#[derive(Template)]
#[template(path = "components/bubble/notification.html")]
pub struct NotificationBubbleResponse<'a> {
    pub message: &'a str,
}
