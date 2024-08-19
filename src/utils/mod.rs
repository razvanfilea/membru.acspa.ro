pub mod date_formats;
pub mod date_iter;
pub mod reservation;

use crate::http::AppState;
use crate::model::location::HourStructure;
use crate::utils::reservation::{ReservationError, ReservationResult, ReservationSuccess};
use sqlx::{query, SqlitePool};
use time::{Date, OffsetDateTime, UtcOffset, Weekday};

pub fn local_time() -> OffsetDateTime {
    let offset = UtcOffset::current_local_offset().expect("Failed to set Soundness to Unsound");
    OffsetDateTime::now_utc().to_offset(offset)
}

pub async fn get_hour_structure_for_day(state: &AppState, date: Date) -> HourStructure {
    if is_free_day(&state.read_pool, date).await {
        state.location.get_alt_hour_structure()
    } else {
        state.location.get_hour_structure()
    }
}

pub async fn is_free_day(pool: &SqlitePool, date: Date) -> bool {
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

    let weekday = date.weekday();
    weekday == Weekday::Saturday || weekday == Weekday::Sunday || exists_in_table.await
}

pub enum CssColor {
    Success,
    Info,
    Warning,
    Error,
}

pub fn get_reservation_result_color(result: &ReservationResult) -> CssColor {
    match result {
        Ok(success) => match success {
            ReservationSuccess::Reservation { .. } => CssColor::Success,
            ReservationSuccess::Guest => CssColor::Info,
        },
        Err(error) => match error {
            ReservationError::AlreadyExists => CssColor::Warning,
            _ => CssColor::Error,
        },
    }
}
