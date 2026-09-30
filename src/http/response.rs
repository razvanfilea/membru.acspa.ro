use crate::http::error::HttpResult;
use askama::Template;
use axum::response::{Html, IntoResponse, Response};

pub fn hx_refresh() -> Response {
    [("HX-Refresh", "true")].into_response()
}

pub fn hx_redirect(path: impl Into<String>) -> Response {
    [("HX-Redirect", path.into())].into_response()
}

pub fn hx_notify(message: impl AsRef<str>) -> HttpResult {
    hx_notify_oob(String::new(), message)
}

/// Returns out-of-band `html` and a success bubble, leaving the request's target untouched
pub fn hx_notify_oob(oob_html: String, message: impl AsRef<str>) -> HttpResult {
    Ok(([("HX-Reswap", "none")], hx_notify_with(oob_html, message)?).into_response())
}

pub fn hx_notify_with(html: String, message: impl AsRef<str>) -> HttpResult {
    #[derive(Template)]
    #[template(path = "components/bubble/success.html")]
    struct SuccessBubbleTemplate<'a> {
        message: &'a str,
    }

    let bubble = SuccessBubbleTemplate {
        message: message.as_ref(),
    }
    .render()?;
    Ok(Html(html + &bubble).into_response())
}
