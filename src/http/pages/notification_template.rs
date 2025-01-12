use crate::http::template_into_response::{HxSwap, TemplateIntoResponse};
use askama::Template;
use axum::response::Response;

pub fn error_bubble_response(message: impl AsRef<str>) -> Response {
    #[derive(Template)]
    #[template(path = "components/bubble/error.html")]
    struct ErrorBubbleTemplate<'a> {
        pub message: &'a str,
    }

    ErrorBubbleTemplate {
        message: message.as_ref(),
    }
    .into_response_swap(HxSwap::None, None)
}

#[derive(Template)]
#[template(path = "components/bubble/notification.html")]
pub struct NotificationBubbleResponse<'a> {
    pub message: &'a str,
}
