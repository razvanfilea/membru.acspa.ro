use crate::http::AppState;
use crate::http::auth::{generate_hash_from_password, validate_credentials};
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::query;
use tracing::debug;

pub async fn change_password_page(auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "user/change_password_page.html")]
    struct ChangePasswordTemplate {
        user: User,
    }

    ChangePasswordTemplate {
        user: auth_session.user.expect("User should be logged in"),
    }
    .into_response()
}

fn change_password_error(message: impl AsRef<str>) -> HttpResult {
    #[derive(Template)]
    #[template(path = "user/login_error.html")]
    struct ErrorTemplate<'a> {
        error_message: &'a str,
    }

    ErrorTemplate {
        error_message: message.as_ref(),
    }
    .try_into_response()
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    old: String,
    new: String,
    new_duplicate: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthSession,
    Form(passwords): Form<ChangePasswordForm>,
) -> HttpResult {
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
    .execute(&state.write_pool)
    .await?;

    debug!("User has been logged in: {}", user.email);
    Ok([("HX-Redirect", "/")].into_response())
}
