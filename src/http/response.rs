use crate::http::error::HttpResult;
use crate::http::pages::notification_template::SuccessBubbleTemplate;
use askama::Template;
use axum::http::{HeaderName, HeaderValue};
use axum::response::{Html, IntoResponse, Response};

/// Returns an HX-Refresh response that tells HTMX to refresh the page
pub fn hx_refresh() -> Response {
    [("HX-Refresh", "true")].into_response()
}

/// Returns an HX-Redirect response that tells HTMX to redirect to the given path
pub fn hx_redirect(path: impl Into<String>) -> Response {
    [("HX-Redirect", path.into())].into_response()
}

/// Returns a success bubble without swapping anything else
pub fn hx_notify(message: impl AsRef<str>) -> HttpResult {
    let mut response = hx_notify_with(String::new(), message)?;
    response.headers_mut().insert(
        const { HeaderName::from_static("hx-reswap") },
        const { HeaderValue::from_static("none") },
    );
    Ok(response)
}

/// Returns `html` (the swapped content) followed by a success bubble
pub fn hx_notify_with(html: String, message: impl AsRef<str>) -> HttpResult {
    let bubble = SuccessBubbleTemplate {
        message: message.as_ref(),
    }
    .render()?;
    Ok(Html(html + &bubble).into_response())
}
