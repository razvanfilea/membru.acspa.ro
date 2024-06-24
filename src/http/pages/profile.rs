use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::BasicUser;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;

#[derive(Template)]
#[template(path = "pages/profile.html")]
struct ProfileTemplate {
    user: BasicUser,
}

pub async fn get_profile(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let user = auth_session.user.unwrap();
    // query_as!("select * reservations");

    ProfileTemplate { user: user.into() }
}
