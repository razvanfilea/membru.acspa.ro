use crate::http::auth::{generate_hash_from_password, validate_credentials};
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::UserUi;
use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::extract::State;
use axum::Form;
use serde::Deserialize;
use sqlx::query;
use tracing::debug;

pub async fn change_password_page(auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/user/change_password.html")]
    struct ChangePasswordTemplate {
        user: UserUi,
    }

    ChangePasswordTemplate {
        user: auth_session.user.expect("User should be logged in"),
    }
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    old: String,
    new: String,
    new_duplicate: String,
}

fn change_password_error(message: impl AsRef<str>) -> Response {
    #[derive(Template)]
    #[template(path = "components/login_error.html")]
    struct ErrorTemplate<'a> {
        error_message: &'a str,
    }

    ErrorTemplate {
        error_message: message.as_ref(),
    }
    .into_response()
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthSession,
    Form(passwords): Form<ChangePasswordForm>,
) -> Response {
    let user = auth.user.as_ref().unwrap();

    if passwords.new != passwords.new_duplicate {
        return change_password_error("Cele 2 parole nu sunt identice");
    }

    if !validate_credentials(passwords.old, user.password_hash.as_str()).unwrap_or(false) {
        return change_password_error("Parola curentă este greșită");
    }

    let new_password_hash = generate_hash_from_password(passwords.new);
    query!(
        "update users set password_hash = $1 where id = $2",
        new_password_hash,
        user.id
    )
    .execute(&state.pool)
    .await
    .expect("Database error");

    debug!("User has been logged in: {}", user.email);
    Response::builder()
        .header("HX-Redirect", "/")
        .body("Ți-ai schimbat parola cu succes".to_string())
        .unwrap()
        .into_response()
}
