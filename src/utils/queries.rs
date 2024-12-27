use crate::http::AppState;
use crate::model::day_structure::{DayStructure, HOLIDAY_DAY_STRUCTURE};
use crate::model::user_reservation::UserReservation;
use sqlx::{query, query_as, Executor, Sqlite, SqlitePool};
use time::{Date, Weekday};
use tracing::error;

pub async fn alt_day_exists<'a, E>(executor: E, date: Date) -> bool
where
    E: Executor<'a, Database = Sqlite>,
{
    query!(
        "select exists (select 1 from alternative_days where date = $1) as 'exists!'",
        date
    )
    .fetch_one(executor)
    .await
    .expect("Database error")
    .exists
        != 0
}

pub async fn get_alt_day_structure_for_day<'a, E>(executor: E, date: Date) -> Option<DayStructure>
where
    E: Executor<'a, Database = Sqlite>,
{
    fn is_weekend(weekday: Weekday) -> bool {
        weekday == Weekday::Saturday || weekday == Weekday::Sunday
    }

    let day = query_as!(
        DayStructure,
        "select slots_start_hour, slot_duration, slots_per_day, description, slot_capacity from alternative_days where date = $1",
        date
    ).fetch_optional(executor).await.expect("Database error");

    day.or_else(|| {
        if is_weekend(date.weekday()) {
            Some(HOLIDAY_DAY_STRUCTURE)
        } else {
            None
        }
    })
}

pub async fn get_day_structure(state: &AppState, date: Date) -> DayStructure {
    get_alt_day_structure_for_day(&state.read_pool, date)
        .await
        .unwrap_or_else(|| state.location.day_structure())
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
