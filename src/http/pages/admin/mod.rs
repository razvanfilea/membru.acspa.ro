use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::global_vars::GlobalVars;
use crate::model::user::User;
use crate::utils::queries::get_global_vars;
use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;

mod guests;
mod members;
mod roles;
mod schedule_overrides;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/apply_settings", post(apply_settings))
        .nest("/members", members::router())
        .nest("/roles", roles::router())
        .nest("/guests", guests::router())
        .merge(schedule_overrides::router())
}

async fn admin_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/admin_page.html")]
    struct AdminTemplate {
        user: User,
        global_vars: GlobalVars,
    }

    AdminTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        global_vars: get_global_vars(&state.read_pool).await?,
    }
    .try_into_response()
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
) -> HttpResult {
    let in_maintenance = settings.in_maintenance.is_some();
    query!(
        "update global_vars set in_maintenance = $1, entrance_code = $2, homepage_message = $3",
        in_maintenance,
        settings.entrance_code,
        settings.homepage_message
    )
    .execute(&state.write_pool)
    .await?;

    Ok("Setările au fost aplicate".into_response())
}
