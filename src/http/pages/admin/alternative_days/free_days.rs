use crate::http::error::HttpResult;
use crate::http::pages::admin::alternative_days::{
    add_alternative_day, alternative_days, delete_alternative_day, AlternativeDay,
    NewAlternativeDay,
};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::http::AppState;
use crate::model::day_structure::HOLIDAY_DAY_STRUCTURE;
use crate::model::user::User;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{delete, get, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{Error, SqlitePool};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(free_days_page))
        .route("/", put(create_free_day))
        .route("/{date}", delete(delete_free_day))
}

async fn get_free_days(pool: &SqlitePool) -> Result<Vec<AlternativeDay>, Error> {
    alternative_days(pool, "holiday").await
}

async fn free_days_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "pages/admin/free_days.html")]
    struct FreeDaysTemplate {
        user: User,
        current_date: Date,
        free_days: Vec<AlternativeDay>,
    }

    FreeDaysTemplate {
        user: auth_session.user.expect("User should be logged in"),
        free_days: get_free_days(&state.read_pool).await?,
        current_date: local_time().date(),
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct NewFreeDay {
    date: String,
    description: Option<String>,
}

async fn create_free_day(
    State(state): State<AppState>,
    Form(new_day): Form<NewFreeDay>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "components/admin/free_days_content.html")]
    struct FreeDaysListTemplate {
        free_days: Vec<AlternativeDay>,
    }

    let day_structure = &HOLIDAY_DAY_STRUCTURE;
    let day = NewAlternativeDay {
        date: new_day.date.clone(),
        description: new_day.description.clone(),
        start_hour: day_structure.slots_start_hour as u8,
        duration: day_structure.slot_duration as u8,
        capacity: None,
        slots_per_day: day_structure.slots_per_day as u8,
    };

    if let Some(error_response) = add_alternative_day(state.write_pool, day, "holiday").await? {
        return Ok(error_response);
    }

    info!(
        "Added free day with date: {} and description {}",
        new_day.date,
        new_day.description.unwrap_or_default()
    );

    FreeDaysListTemplate {
        free_days: get_free_days(&state.read_pool).await?,
    }
    .try_into_response()
}

pub async fn delete_free_day(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> HttpResult {
    delete_alternative_day(state, date).await
}
