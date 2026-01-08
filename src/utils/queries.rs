use crate::http::AppState;
use crate::model::day_structure::{DayStructure, HOLIDAY_DAY_STRUCTURE};
use crate::model::global_vars::GlobalVars;
use crate::model::location::Location;
use crate::model::user::User;
use crate::model::user_reservation::UserReservation;
use itertools::Itertools;
use sqlx::{SqliteExecutor, SqlitePool, query, query_as};
use time::{Date, Month, Weekday};
use tracing::error;

pub async fn get_global_vars(pool: &SqlitePool) -> sqlx::Result<GlobalVars> {
    query_as!(
        GlobalVars,
        "select in_maintenance, entrance_code, homepage_message from global_vars"
    )
    .fetch_one(pool)
    .await
}

pub async fn get_user(pool: &SqlitePool, id: i64) -> sqlx::Result<User> {
    query_as!(User, "select * from users_with_role where id = $1", id)
        .fetch_one(pool)
        .await
}

pub async fn get_alt_day_structure_for_day(
    executor: impl SqliteExecutor<'_>,
    date: Date,
) -> Option<DayStructure> {
    fn is_weekend(weekday: Weekday) -> bool {
        weekday == Weekday::Saturday || weekday == Weekday::Sunday
    }

    let day = query_as!(
        DayStructure,
        "select slots_start_hour, slots_start_minute, slot_duration, slots_per_day, description, slot_capacity, consumes_reservation
         from alternative_days where date = $1",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YearMonth {
    pub year: i32,
    pub month: Month,
}

impl YearMonth {
    pub fn new(year: i32, month: Month) -> Self {
        Self { year, month }
    }
}

pub struct GroupedUserReservations {
    pub year: i32,
    pub month: Month,
    pub reservations: Vec<UserReservation>,
}

pub async fn get_user_reservations(
    pool: &SqlitePool,
    user_id: i64,
    cancelled: bool,
) -> Vec<GroupedUserReservations> {
    let reservations = query_as!(
        UserReservation,
        "select r.date, r.hour, r.as_guest, r.cancelled, r.in_waiting, r.created_at from reservations as r
         where user_id = $1 and cancelled = $2 and created_for is null",
        user_id,
        cancelled
    ).fetch_all(pool)
        .await
        .inspect_err(|e| error!("Failed querying reservations for user: {e}"))
        .unwrap_or_default();

    reservations
        .into_iter()
        .into_group_map_by(|res| YearMonth {
            year: res.date.year(),
            month: res.date.month(),
        })
        .into_iter()
        .map(|(year_month, reservations)| GroupedUserReservations {
            year: year_month.year,
            month: year_month.month,
            reservations,
        })
        .sorted_by(|a, b| {
            a.year
                .cmp(&b.year)
                .then((a.month as u8).cmp(&(b.month as u8)))
                .reverse()
        })
        .collect()
}

#[derive(Debug, Default)]
pub struct ReservationsCount {
    pub member: i64,
    pub guest: i64,
}

pub async fn get_reservations_count_for_slot(
    executor: impl SqliteExecutor<'_>,
    location: &Location,
    date: Date,
    hour: u8,
) -> sqlx::Result<ReservationsCount> {
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
    executor: impl SqliteExecutor<'_>,
    user: &User,
    date: Date,
) -> sqlx::Result<ReservationsCount> {
    let counts = query!(
        "select r.as_guest, count(*) as 'count! :i64' from reservations r
         left join alternative_days d on r.date = d.date
         where r.user_id = $1 and r.cancelled = false
         and (d.consumes_reservation is null or d.consumes_reservation = true)
         and strftime('%Y%W', r.date) = strftime('%Y%W', $2)
         group by r.as_guest",
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

pub async fn delete_reservations_on_day(
    executor: impl SqliteExecutor<'_>,
    date: Date,
    hour: Option<u8>,
) -> sqlx::Result<u64> {
    query!(
        "delete from reservations where date = $1 and ($2 is null or hour = $2)",
        date,
        hour
    )
    .execute(executor)
    .await
    .map(|result| result.rows_affected())
}
