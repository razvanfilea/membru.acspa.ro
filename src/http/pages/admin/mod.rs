use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::user::UserUi;

mod members;
mod roles;
mod free_days;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/apply_settings", post(apply_settings))
        .nest("/members", members::router())
        .nest("/roles", roles::router())
        .nest("/free_days", free_days::router())
}

async fn get_global_vars(state: &AppState) -> GlobalVars {
    query_as!(GlobalVars, "select * from global_vars")
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
}

async fn admin_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/index.html")]
    struct HomeTemplate {
        user: UserUi,
        global_vars: GlobalVars,
    }

    HomeTemplate {
        user: auth_session.user.unwrap().into(),
        global_vars: get_global_vars(&state).await,
    }
}

#[derive(Deserialize)]
struct NewSettings {
    in_maintenance: Option<String>,
    entrance_code: String,
    reminder_message: String,
}

async fn apply_settings(
    State(state): State<AppState>,
    Form(settings): Form<NewSettings>,
) -> impl IntoResponse {
    let in_maintenance = settings.in_maintenance.is_some();
    query!(
        "update global_vars set in_maintenance = $1, entrance_code = $2, reminder_message = $3",
        in_maintenance,
        settings.entrance_code,
        settings.reminder_message
    )
    .execute(&state.pool)
    .await
    .expect("Database error");

    "Setările au fost aplicate"
}
