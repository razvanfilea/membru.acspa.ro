mod reservation;

use crate::http::AppState;
use crate::model::location::HourStructure;
use chrono::{Datelike, NaiveDate, Weekday};
use sqlx::{query, SqlitePool};

pub async fn get_hour_structure_for_day(state: &AppState, date: &NaiveDate) -> HourStructure {
    if is_free_day(&state.pool, &date).await {
        state.location.get_alt_hour_structure()
    } else {
        state.location.get_hour_structure()
    }
}

pub async fn is_free_day(pool: &SqlitePool, date: &NaiveDate) -> bool {
    let exists_in_table = async {
        query!(
            "select exists(select true from free_days where date = $1) as 'exists!'",
            date
        )
        .fetch_one(pool)
        .await
        .expect("Database error")
        .exists
            != 0
    };

    date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun || exists_in_table.await
}
