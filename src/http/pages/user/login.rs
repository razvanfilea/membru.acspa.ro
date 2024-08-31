use askama::Template;
use askama_axum::IntoResponse;
use axum::response::{Redirect, Response};
use axum::Form;
use tracing::{debug, error};
use validator::Validate;

use crate::http::pages::AuthSession;
use crate::model::user::UserCredentials;

pub async fn login_page(auth_session: AuthSession) -> impl IntoResponse {
    if auth_session.user.is_some() {
        return Redirect::to("/").into_response();
    }

    #[derive(Template)]
    #[template(path = "pages/login.html")]
    struct LoginTemplate;

    LoginTemplate.into_response()
}

fn login_error(message: impl AsRef<str>) -> Response {
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

pub async fn login(
    mut auth: AuthSession,
    Form(login_user): Form<UserCredentials>,
) -> impl IntoResponse {
    let generic_error_template = login_error(
        "Serverul a întâmpinat o problemă, dacă eroare persistă te rog contactează un membru fondator",
    );

    if let Err(e) = login_user.validate() {
        return login_error(e.to_string());
    }

    let user = match auth.authenticate(login_user.clone()).await {
        Ok(user) => {
            if let Some(user) = user {
                user
            } else {
                return login_error("Email sau parolă invalidă");
            }
        }
        Err(e) => {
            error!(
                "Failed to authenticate user {} with error: {}",
                login_user.email, e
            );
            return generic_error_template;
        }
    };

    match auth.login(&user).await {
        Ok(()) => {
            debug!("User has been logged in: {}", user.email);
            Response::builder()
                .header("HX-Replace-Url", "/")
                .header("HX-Refresh", "true")
                .body("Ai fost logat cu succes".to_string())
                .unwrap()
                .into_response()
        }
        Err(e) => {
            error!("Failed to login user {} with error: {}", user.email, e);
            generic_error_template
        }
    }
}

pub async fn logout(mut auth: AuthSession) -> impl IntoResponse {
    if let Some(user) = &auth.user {
        debug!("Logging out user: {}", user.id);

        if let Err(e) = auth.logout().await {
            error!("Failed to log out user: {e}");
        }
    }

    Response::builder()
        .header("HX-Redirect", "/login")
        .body("Ai fost de-logat cu succes".to_string())
        .unwrap()
}
