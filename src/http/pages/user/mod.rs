use crate::http::AppState;
use crate::http::template_into_response::TemplateIntoResponse;
use askama::Template;
use axum::Router;
use axum::response::IntoResponse;
use axum::routing::{get, post};

mod change_password;
pub mod login;
mod profile;

pub fn user_router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(profile::profile_page))
        .route("/profile/reservations", post(profile::profile_reservations))
        .route(
            "/profile/payment_status/{year}",
            get(profile::payment_status_partial),
        )
        .route(
            "/change_password",
            get(change_password::change_password_page),
        )
        .route("/change_password", post(change_password::change_password))
}

pub async fn forgot_password() -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "user/forgot_password_page.html")]
    struct ForgotPassword;

    ForgotPassword.into_response()
}
