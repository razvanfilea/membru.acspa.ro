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
    struct LoginTemplate {
        error: Option<&'static str>,
    }

    LoginTemplate { error: None }.into_response()
}

#[derive(Template)]
#[template(path = "components/login_form.html")]
pub struct LoginTemplate {
    error: Option<String>,
}

impl LoginTemplate {
    fn new(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
        }
    }
}

pub async fn login(
    mut auth: AuthSession,
    Form(login_user): Form<UserCredentials>,
) -> Result<impl IntoResponse, LoginTemplate> {
    let generic_error_template = LoginTemplate::new(
        "Serverul are probleme, dacă eroare persistă te rog contactează un membru fondator",
    );

    if let Err(e) = login_user.validate() {
        return Err(LoginTemplate::new(e.to_string()));
    }

    let user = match auth.authenticate(login_user.clone()).await {
        Ok(user) => {
            if let Some(user) = user {
                user
            } else {
                return Err(LoginTemplate::new("Email sau parolă invalidă"));
            }
        }
        Err(e) => {
            error!(
                "Failed to authenticate user {} with error: {}",
                login_user.email, e
            );
            return Err(generic_error_template);
        }
    };

    match auth.login(&user).await {
        Ok(()) => {
            debug!("User has been logged in: {}", user.email);
            Response::builder()
                .header("HX-Redirect", "/")
                .body("Ai fost logat cu succes".to_string())
                .map_err(|e| {
                    error!("Failed to return headers: {e}");
                    generic_error_template
                })
        }
        Err(e) => {
            error!("Failed to login user {} with error: {}", user.email, e);
            Err(generic_error_template)
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
        .header("HX-Redirect", "/")
        .body("Ai fost de-logat cu succes".to_string())
        .unwrap()
}
