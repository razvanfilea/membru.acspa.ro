mod color;
pub mod date_formats;
pub mod date_iter;
pub mod reservation;

use crate::http::AppState;
use crate::model::location::HourStructure;
use crate::utils::reservation::{ReservationError, ReservationResult, ReservationSuccess};
pub use color::*;
use sqlx::{query, query_as, Executor, Sqlite, SqlitePool};
use time::{Date, OffsetDateTime, UtcOffset, Weekday};
use tracing::error;
use crate::model::user_reservation::UserReservation;

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

pub async fn is_free_day<'a, E>(executor: E, date: Date) -> bool
where
    E: Executor<'a, Database = Sqlite>,
{
    let exists_in_table = async {
        query!(
            "select exists(select true from free_days where date = $1) as 'exists!'",
            date
        )
        .fetch_one(executor)
        .await
        .expect("Database error")
        .exists
            != 0
    };

    let weekday = date.weekday();
    weekday == Weekday::Saturday || weekday == Weekday::Sunday || exists_in_table.await
}

pub fn get_reservation_result_color(result: &ReservationResult) -> CssColor {
    match result {
        Ok(success) => match success {
            ReservationSuccess::Reservation { .. } => CssColor::Green,
            ReservationSuccess::Guest => CssColor::Blue,
            ReservationSuccess::InWaiting => CssColor::Blue,
        },
        Err(error) => match error {
            ReservationError::AlreadyExists { .. } => CssColor::Yellow,
            _ => CssColor::Red,
        },
    }
}

pub async fn get_user_reservations(
    pool: &SqlitePool,
    email: &str,
    cancelled: bool,
) -> Vec<UserReservation> {
    query_as!(
        UserReservation,
        "select r.date, r.hour, r.as_guest, r.cancelled, r.in_waiting, r.created_at from reservations as r inner join users on user_id = users.id where email = $1 and cancelled = $2 and created_for is null order by date desc, hour asc",
        email,
        cancelled
    ).fetch_all(pool)
        .await
        .inspect_err(|e| error!("Failed querying reservations for user: {e}"))
        .unwrap_or_default()
}

