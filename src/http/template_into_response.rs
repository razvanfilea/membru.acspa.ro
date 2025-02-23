use crate::http::error::HttpResult;
use axum::http::{HeaderMap, HeaderName, header};
use axum::response::IntoResponse;
use axum::response::Response;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum HxSwap {
    InnerHtml,
    OuterHtml,
    TextContent,
    BeforeBegin,
    AfterBegin,
    BeforeEnd,
    AfterEnd,
    Delete,
    None,
}

impl HxSwap {
    fn to_str(self) -> &'static str {
        match self {
            HxSwap::InnerHtml => "innerHTML",
            HxSwap::OuterHtml => "outerHTML",
            HxSwap::TextContent => "textContent",
            HxSwap::BeforeBegin => "beforebegin",
            HxSwap::AfterBegin => "afterbegin",
            HxSwap::BeforeEnd => "beforeend",
            HxSwap::AfterEnd => "afterend",
            HxSwap::Delete => "delete",
            HxSwap::None => "none",
        }
    }
}

pub trait TemplateIntoResponse {
    const CONTENT_TYPE_HEADER: (axum::http::HeaderName, &'static str) =
        (header::CONTENT_TYPE, "text/html; charset=utf-8");

    fn try_into_response(self) -> HttpResult;

    fn into_response(self) -> Response;

    fn into_response_swap(self, swap: HxSwap, target: Option<&str>) -> Response;
}

impl<T> TemplateIntoResponse for T
where
    T: askama::Template,
{
    fn try_into_response(self) -> HttpResult {
        let body = self.render()?;
        Ok(([Self::CONTENT_TYPE_HEADER], body).into_response())
    }

    fn into_response(self) -> Response {
        let body = self.render().expect("Failed to render template");
        ([Self::CONTENT_TYPE_HEADER], body).into_response()
    }

    fn into_response_swap(self, swap: HxSwap, target: Option<&str>) -> Response {
        let body = self.render().expect("Failed to render template");

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            Self::CONTENT_TYPE_HEADER.1.parse().unwrap(),
        );
        headers.insert(
            HeaderName::from_static("hx-reswap"),
            swap.to_str().parse().unwrap(),
        );

        if let Some(target) = target {
            headers.insert(
                HeaderName::from_static("hx-retarget"),
                target.parse().expect("Failed to parse HX target"),
            );
        }

        (headers, body).into_response()
    }
}
