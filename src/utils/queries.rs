use crate::http::AppState;
use crate::model::day_structure::{DayStructure, HOLIDAY_DAY_STRUCTURE};
use crate::model::location::Location;
use crate::model::user::User;
use crate::model::user_reservation::UserReservation;
use crate::reservation::ReservationResult;
use sqlx::{query, query_as, Executor, Sqlite, SqlitePool};
use time::{Date, Weekday};
use tracing::error;

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

#[derive(Debug, Default)]
pub struct ReservationsCount {
    pub member: i64,
    pub guest: i64,
}

pub async fn get_current_reservations_count(
    executor: impl Executor<'_, Database = Sqlite>,
    location: &Location,
    date: Date,
    hour: u8,
) -> ReservationResult<ReservationsCount> {
    let counts = query!(
        "select as_guest, count(*) as 'count!: i64' from reservations
        where location = $1 and date = $2 and hour = $3 and cancelled = false and in_waiting = false
        group by as_guest",
        location.id,
        date,
        hour
    )
    .fetch_all(executor)
    .await?;

    let mut result = ReservationsCount::default();

    for row in counts {
        if row.as_guest {
            result.guest = row.count;
        } else {
            result.member = row.count;
        }
    }

    Ok(result)
}

pub async fn get_user_weeks_reservations_count(
    executor: impl Executor<'_, Database = Sqlite>,
    user: &User,
    date: Date,
) -> Result<ReservationsCount, sqlx::Error>
{
    let counts = query!(
        "select as_guest, count(*) as 'count! :i64' from reservations
        where user_id = $1 and cancelled = false and
        strftime('%Y%W', date) = strftime('%Y%W', $2)
        group by as_guest",
        user.id,
        date
    )
    .fetch_all(executor)
    .await?;

    let mut result = ReservationsCount::default();

    for row in counts {
        if row.as_guest {
            result.guest = row.count;
        } else {
            result.member = row.count;
        }
    }

    Ok(result)
}
