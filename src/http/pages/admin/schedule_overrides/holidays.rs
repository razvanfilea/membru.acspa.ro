use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::admin::schedule_overrides::calendar::day_details_response;
use crate::http::pages::admin::schedule_overrides::{
    AlternativeDay, AlternativeDayType, NewAlternativeDay, add_alternative_day,
    delete_alternative_day, get_alternative_day, get_alternative_days,
};
use crate::model::day_structure::HOLIDAY_DAY_STRUCTURE;
use crate::utils::date_formats;
use axum::extract::{Path, State};
use axum::routing::{delete, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{Error, SqlitePool};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", put(create_holiday))
        .route("/{date}", delete(delete_holiday))
}
pub async fn get_holiday(pool: &SqlitePool, date: Date) -> Result<Option<AlternativeDay>, Error> {
    get_alternative_day(pool, AlternativeDayType::Holiday, date).await
}

pub async fn get_holidays_for_month(
    pool: &SqlitePool,
    month_year: Date,
) -> Result<Vec<AlternativeDay>, Error> {
    get_alternative_days(pool, AlternativeDayType::Holiday, Some(month_year)).await
}

#[derive(Deserialize)]
struct NewHoliday {
    date: String,
    description: Option<String>,
}

async fn create_holiday(
    State(state): State<AppState>,
    Form(new_day): Form<NewHoliday>,
) -> HttpResult {
    let date = Date::parse(&new_day.date, date_formats::ISO_DATE).expect("Invalid date");

    let day_structure = &HOLIDAY_DAY_STRUCTURE;
    let day = NewAlternativeDay {
        date,
        description: new_day.description.clone(),
        start_hour: day_structure.slots_start_hour as u8,
        start_minute: 0,
        duration: day_structure.slot_duration as u8,
        capacity: None,
        slots_per_day: day_structure.slots_per_day as u8,
        consumes_reservation: true,
    };

    add_alternative_day(&state.write_pool, day, AlternativeDayType::Holiday).await?;

    info!(
        "Added free day with date: {} and description {}",
        new_day.date,
        new_day.description.unwrap_or_default()
    );

    day_details_response(state, date).await
}

async fn delete_holiday(State(state): State<AppState>, Path(date_str): Path<String>) -> HttpResult {
    let date = Date::parse(&date_str, date_formats::ISO_DATE).expect("Invalid date");
    delete_alternative_day(&state, date_str).await?;
    day_details_response(state, date).await
}
