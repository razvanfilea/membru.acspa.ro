use askama::Template;
use askama_axum::IntoResponse;
use crate::http::pages::AuthSession;
use crate::model::user::UserUi;
use crate::utils::date_formats;

pub async fn change_password_page(auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/change_password.html")]
    struct ChangePasswordTemplate {
        user: UserUi,
        error: Option<&'static str>,
    }

    ChangePasswordTemplate {
        user: auth_session.user.unwrap(),
        error: None,
    }
}
