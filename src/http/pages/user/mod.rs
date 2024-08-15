use askama::Template;
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::{get, post};
use crate::http::AppState;

pub mod login;
mod profile;
mod change_password;

pub fn user_router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(profile::profile_page))
        .route("/profile/reservations", post(profile::profile_reservations))
        .route("/change_password", get(change_password::change_password_page))
        .route("/change_password", post(change_password::change_password))
}

pub async fn forgot_password() -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/forgot_password.html")]
    struct ForgotPassword;

    ForgotPassword
}