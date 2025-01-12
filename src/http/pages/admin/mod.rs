use crate::http::pages::{get_global_vars, AuthSession};
use crate::http::template_into_response::TemplateIntoResponse;
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::user::User;
use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;

mod alternative_days;
mod guests;
mod members;
mod restrictions;
mod roles;
mod situations;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/apply_settings", post(apply_settings))
        .nest("/members", members::router())
        .nest("/roles", roles::router())
        .nest("/restrictions", restrictions::router())
        .nest("/guests", guests::router())
        .nest("/situations", situations::router())
        .merge(alternative_days::router())
}

async fn admin_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/admin.html")]
    struct HomeTemplate {
        user: User,
        global_vars: GlobalVars,
    }

    HomeTemplate {
        user: auth_session.user.expect("User should be logged in"),
        global_vars: get_global_vars(&state).await,
    }
    .into_response()
}

#[derive(Deserialize)]
struct NewSettings {
    in_maintenance: Option<String>,
    entrance_code: String,
    homepage_message: String,
}

async fn apply_settings(
    State(state): State<AppState>,
    Form(settings): Form<NewSettings>,
) -> impl IntoResponse {
    let in_maintenance = settings.in_maintenance.is_some();
    query!(
        "update global_vars set in_maintenance = $1, entrance_code = $2, homepage_message = $3",
        in_maintenance,
        settings.entrance_code,
        settings.homepage_message
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    "Setările au fost aplicate"
}
