use crate::http::AppState;
use crate::http::error::{HttpResult, bail};
use crate::model::user_reservation::UserReservation;
use crate::utils::date_formats::DateFormatExt;
use axum::Router;
use sqlx::{SqliteConnection, SqliteExecutor, SqlitePool, query, query_as, query_scalar};
use time::{Date, OffsetDateTime};
use tracing::info;

mod calendar;
mod holidays;
mod restrictions;
mod tournaments;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/calendar", calendar::router())
        .nest("/tournaments", tournaments::router())
        .nest("/holiday", holidays::router())
        .nest("/restrictions", restrictions::router())
}

struct NewAlternativeDay {
    date: Date,
    description: Option<String>,
    start_hour: u8,
    start_minute: u8,
    duration: u8,
    slots_per_day: u8,
    capacity: Option<u8>,
    consumes_reservation: bool,
}

async fn add_alternative_day(
    write_pool: &SqlitePool,
    day: NewAlternativeDay,
    day_type: AlternativeDayType,
) -> HttpResult<()> {
    let mut tx = write_pool.begin().await?;

    if alt_day_exists(tx.as_mut(), day.date).await? {
        return Err(bail(format!(
            "Deja exists o zi libera/turneu pe data de {}",
            day.date.to_readable()
        )));
    }

    let day_type = day_type.as_ref();
    let description = day
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());
    let start_minute = Some(day.start_minute).filter(|minute| *minute > 0 && *minute < 60);

    query!(
        "insert into alternative_days (type, date, description, slots_start_hour, slots_start_minute,
         slot_duration, slot_capacity, slots_per_day, consumes_reservation)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        day_type,
        day.date,
        description,
        day.start_hour,
        start_minute,
        day.duration,
        day.capacity,
        day.slots_per_day,
        day.consumes_reservation
    )
    .execute(tx.as_mut())
    .await?;

    let deleted_reservations = UserReservation::delete_on_day(tx.as_mut(), day.date, None).await?;
    if deleted_reservations != 0 {
        info!("{deleted_reservations} reservation were deleted when creating alternative day");
    }

    tx.commit().await?;

    Ok(())
}

#[derive(Clone)]
struct AlternativeDay {
    date: Date,
    description: String,
    start_hour: i64,
    start_minute: Option<i64>,
    duration: i64,
    slot_capacity: Option<i64>,
    consumes_reservation: bool,
    created_at: OffsetDateTime,
}

#[derive(Clone, Copy)]
enum AlternativeDayType {
    Holiday,
    Tournament,
}

impl AsRef<str> for AlternativeDayType {
    fn as_ref(&self) -> &str {
        match self {
            AlternativeDayType::Holiday => "holiday",
            AlternativeDayType::Tournament => "turneu",
        }
    }
}

async fn get_alternative_day(
    executor: impl SqliteExecutor<'_>,
    day_type: AlternativeDayType,
    date: Date,
) -> sqlx::Result<Option<AlternativeDay>> {
    let day_type = day_type.as_ref();
    query_as!(AlternativeDay, "select date, COALESCE(description, '') as 'description!: String',
        slots_start_hour as 'start_hour', slot_duration as 'duration', slot_capacity, consumes_reservation, slots_start_minute as 'start_minute', created_at
        from alternative_days where type = $1 and date = $2", day_type, date)
        .fetch_optional(executor)
        .await
}

async fn get_alternative_days(
    pool: &SqlitePool,
    day_type: AlternativeDayType,
    month_year: Option<Date>,
) -> sqlx::Result<Vec<AlternativeDay>> {
    let day_type = day_type.as_ref();
    query_as!(AlternativeDay, "select date, COALESCE(description, '') as 'description',
        slots_start_hour as 'start_hour', slot_duration as 'duration', slot_capacity, consumes_reservation, slots_start_minute as 'start_minute', created_at
        from alternative_days where type = $1
        and strftime('%m%Y', date) = strftime('%m%Y', COALESCE($2, date))
        order by date desc, created_at", day_type, month_year)
        .fetch_all(pool)
        .await
}

async fn delete_alternative_day(state: &AppState, date: Date) -> HttpResult<()> {
    let mut tx = state.write_pool.begin().await?;

    let deleted_reservations = UserReservation::delete_on_day(tx.as_mut(), date, None).await?;
    if deleted_reservations != 0 {
        info!("{deleted_reservations} reservation were deleted when deleting alternative day");
    }

    query!("delete from alternative_days where date = $1", date)
        .execute(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(())
}

async fn alt_day_exists(conn: &mut SqliteConnection, date: Date) -> sqlx::Result<bool> {
    Ok(query_scalar!(
        "select exists (select 1 from alternative_days where date = $1) as 'exists!'",
        date
    )
    .fetch_one(conn)
    .await?
        != 0)
}
