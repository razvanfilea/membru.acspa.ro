use crate::http::error::HttpResult;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;

pub trait TemplateIntoResponse {
    const CONTENT_TYPE_HEADER: (axum::http::HeaderName, &'static str) =
        (header::CONTENT_TYPE, "text/html; charset=utf-8");

    fn try_into_response(self) -> HttpResult;

    fn into_response(self) -> Response;
}

impl<T> TemplateIntoResponse for T
where
    T: askama::Template,
{
    fn try_into_response(self) -> HttpResult {
        let value = self.render()?;
        Ok(([Self::CONTENT_TYPE_HEADER], value).into_response())
    }

    fn into_response(self) -> Response {
        let value = self.render().expect("Failed to render template");
        ([Self::CONTENT_TYPE_HEADER], value).into_response()
    }
}
