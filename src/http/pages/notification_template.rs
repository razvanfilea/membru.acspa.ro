use crate::http::template_into_response::TemplateIntoResponse;
use askama::Template;
use axum::http::{HeaderName, HeaderValue};
use axum::response::Response;

pub fn error_bubble_response(message: impl AsRef<str>) -> Response {
    #[derive(Template)]
    #[template(path = "components/bubble/error.html")]
    struct ErrorBubbleTemplate<'a> {
        pub message: &'a str,
    }

    let mut response = ErrorBubbleTemplate {
        message: message.as_ref(),
    }
    .into_response();

    response.headers_mut().insert(
        const { HeaderName::from_static("hx-reswap") },
        const { HeaderValue::from_static("none") },
    );

    response
}

#[derive(Template)]
#[template(path = "components/bubble/notification.html")]
pub struct NotificationBubbleResponse<'a> {
    pub message: &'a str,
}
