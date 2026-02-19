use axum::response::{IntoResponse, Response};

/// Returns an HX-Refresh response that tells HTMX to refresh the page
pub fn hx_refresh() -> Response {
    [("HX-Refresh", "true")].into_response()
}

/// Returns an HX-Redirect response that tells HTMX to redirect to the given path
pub fn hx_redirect(path: impl Into<String>) -> Response {
    [("HX-Redirect", path.into())].into_response()
}
