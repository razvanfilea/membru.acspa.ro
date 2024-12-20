use crate::http::AppState;
use crate::model::hour_structure::{HourStructure, HOLIDAY_HOUR_STRUCTURE};
use crate::model::user_reservation::UserReservation;
use sqlx::{query_as, Executor, Sqlite, SqlitePool};
use time::{Date, Weekday};
use tracing::error;

fn is_weekend(weekday: Weekday) -> bool {
    weekday == Weekday::Saturday || weekday == Weekday::Sunday
}

pub async fn get_alt_hour_structure_for_day<'a, E>(executor: E, date: Date) -> Option<HourStructure>
where
    E: Executor<'a, Database = Sqlite>,
{
    if is_weekend(date.weekday()) {
        Some(HOLIDAY_HOUR_STRUCTURE)
    } else {
        query_as!(
            HourStructure,
            "select slots_start_hour, slot_duration, slots_per_day, description from alternative_days where date = $1",
            date
        ).fetch_optional(executor).await.expect("Database error")
    }
}

pub async fn get_hour_structure_for_day(state: &AppState, date: Date) -> HourStructure {
    get_alt_hour_structure_for_day(&state.read_pool, date)
        .await
        .unwrap_or_else(|| state.location.hour_structure())
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
