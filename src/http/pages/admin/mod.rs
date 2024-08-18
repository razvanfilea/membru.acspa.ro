use crate::http::pages::{get_global_vars, AuthSession};
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::user::UserUi;
use crate::utils::date_formats::{ISO_DATE_UNDERLINE, READABLE_DATE_TIME};
use crate::utils::local_time;
use askama::Template;
use askama_axum::Response;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;

mod free_days;
mod guests;
mod members;
mod restrictions;
mod roles;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/apply_settings", post(apply_settings))
        .nest("/members", members::router())
        .nest("/roles", roles::router())
        .nest("/free_days", free_days::router())
        .nest("/restrictions", restrictions::router())
        .nest("/guests", guests::router())
        .route("/situations", get(download_situations))
}

async fn admin_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/admin.html")]
    struct HomeTemplate {
        user: UserUi,
        global_vars: GlobalVars,
    }

    HomeTemplate {
        user: auth_session.user.expect("User should be logged in"),
        global_vars: get_global_vars(&state).await,
    }
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
    .execute(&state.pool)
    .await
    .expect("Database error");

    "Setările au fost aplicate"
}

async fn download_situations(State(state): State<AppState>) -> impl IntoResponse {
    let current_date = local_time().date().format(ISO_DATE_UNDERLINE).unwrap();

    let mut situations: Vec<_> = query!(
        "select r.*, u.name from reservations r join users u on r.user_id = u.id order by date, hour, created_at"
    )
    .fetch_all(&state.pool)
    .await
    .expect("Database error")
    .into_iter()
    .map(|res| {
        format!(
            "{}, \"{}\", {}, {}, {}, {}, \"{}\", \"{}\"",
            res.user_id,
            res.name,
            res.date,
            res.hour,
            res.as_guest,
            res.cancelled,
            res.created_for.unwrap_or_default(),
            res.created_at.format(READABLE_DATE_TIME).unwrap()
        )
    })
    .collect();

    situations.insert(
        0,
        "User ID, Nume, Data, Ora, Ca invitat, Anulat, Creat pentru, Creat pe".to_string(),
    );

    let csv = situations.join("\n");

    Response::builder()
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"situatie_{current_date}.csv\""),
        )
        .body(csv)
        .expect("Failed to create response")
}
