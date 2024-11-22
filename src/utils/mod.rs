mod color;
pub mod date_formats;
pub mod date_iter;
pub mod reservation;

use crate::http::AppState;
use crate::model::location::HourStructure;
use crate::model::user_reservation::UserReservation;
pub use color::*;
use sqlx::{query_as, Executor, Row, Sqlite, SqlitePool};
use time::{Date, OffsetDateTime, UtcOffset, Weekday};
use tracing::error;

pub fn local_time() -> OffsetDateTime {
    let offset = UtcOffset::current_local_offset().expect("Failed to set Soundness to Unsound");
    OffsetDateTime::now_utc().to_offset(offset)
}

fn is_weekend(weekday: Weekday) -> bool {
    weekday == Weekday::Saturday || weekday == Weekday::Sunday
}

pub async fn get_alt_hour_structure_for_day<'a, E>(
    executor: E,
    date: Date,
    alternative_hour_structure: HourStructure,
) -> Option<HourStructure>
where
    E: Executor<'a, Database = Sqlite>,
{
    if is_weekend(date.weekday()) {
        Some(alternative_hour_structure)
    } else {
        query_as!(
            HourStructure,
            "select slots_start_hour, slot_duration, slots_per_day from alternative_days where date = $1",
            date
        ).fetch_optional(executor).await.expect("Database error")
    }
}

pub async fn get_hour_structure_for_day(state: &AppState, date: Date) -> HourStructure {
    get_alt_hour_structure_for_day(
        &state.read_pool,
        date,
        state.alternative_hour_structure.clone(),
    )
    .await
    .unwrap_or_else(|| state.location.get_hour_structure())
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

pub async fn get_default_alt_hour_structure<'a, E>(executor: E) -> HourStructure
where
    E: Executor<'a, Database = Sqlite>,
{
    let mut slots_start_hour = 0;
    let mut slot_duration = 0;
    let mut slots_per_day = 0;

    let rows = sqlx::query(
        "select name, dflt_value FROM pragma_table_info('alternative_days') where name in ('slots_start_hour', 'slot_duration', 'slots_per_day')")
        .fetch_all(executor)
        .await
        .expect("Cannot load default hour structures");

    for row in rows {
        let name: String = row.try_get("name").expect("name does not exist");
        let value: String = row
            .try_get("dflt_value")
            .expect("dflt_value does not exist");
        let value: i64 = value.parse().expect("dflt_value must be an int");

        match name.as_str() {
            "slots_start_hour" => slots_start_hour = value,
            "slot_duration" => slot_duration = value,
            "slots_per_day" => slots_per_day = value,
            _ => panic!("Invalid row"),
        }
    }

    HourStructure {
        slots_start_hour,
        slot_duration,
        slots_per_day,
    }
}
