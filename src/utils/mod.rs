pub mod reservation;
pub mod date_formats;

use crate::http::AppState;
use crate::model::location::HourStructure;
use sqlx::{query, SqlitePool};
use time::{Date, OffsetDateTime, UtcOffset, Weekday};

pub fn local_time() -> OffsetDateTime {
    let offset = UtcOffset::current_local_offset().expect("Failed to set Soundness to Unsound");
    OffsetDateTime::now_utc().to_offset(offset)
}

pub async fn get_hour_structure_for_day(state: &AppState, date: &Date) -> HourStructure {
    if is_free_day(&state.pool, &date).await {
        state.location.get_alt_hour_structure()
    } else {
        state.location.get_hour_structure()
    }
}

pub async fn is_free_day(pool: &SqlitePool, date: &Date) -> bool {
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

    date.weekday() == Weekday::Saturday || date.weekday() == Weekday::Sunday || exists_in_table.await
}
