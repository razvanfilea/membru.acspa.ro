use crate::http::error::HttpResult;
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::AppState;
use crate::utils::date_formats;
use axum::response::{IntoResponse, Response};
use axum::Router;
use sqlx::{query, query_as, Executor, Sqlite, SqliteConnection, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::info;

mod free_days;
mod tournaments;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/free_days", free_days::router())
        .nest("/tournaments", tournaments::router())
}

struct NewAlternativeDay {
    date: String,
    description: Option<String>,
    start_hour: u8,
    duration: u8,
    slots_per_day: u8,
    capacity: Option<u8>,
}

async fn add_alternative_day(
    write_pool: SqlitePool,
    day: NewAlternativeDay,
    day_type: &str,
) -> HttpResult<Option<Response>> {
    let Ok(date) = Date::parse(&day.date, date_formats::ISO_DATE) else {
        return Ok(Some(error_bubble_response("Data selectată nu este valida")));
    };

    let mut tx = write_pool.begin().await?;

    if alt_day_exists(tx.as_mut(), date).await? {
        return Ok(Some(error_bubble_response(format!(
            "Deja exists o zi libera/turneu pe data de {}",
            date.format(date_formats::READABLE_DATE).unwrap()
        ))));
    }

    let description = day
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());

    query!(
        "insert into alternative_days (type, date, description, slots_start_hour, slot_duration, slot_capacity, slots_per_day) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        day_type,
        date,
        description,
        day.start_hour,
        day.duration,
        day.capacity,
        day.slots_per_day
    )
        .execute(tx.as_mut())
        .await?;

    let deleted_reservations = delete_reservations_on_day(tx.as_mut(), date).await?;
    if deleted_reservations != 0 {
        info!("{deleted_reservations} reservation were deleted when creating alternative day");
    }

    tx.commit().await?;

    Ok(None)
}

struct AlternativeDay {
    date: Date,
    description: String,
    start_hour: i64,
    duration: i64,
    slot_capacity: Option<i64>,
    created_at: OffsetDateTime,
}

async fn alternative_days(pool: &SqlitePool, day_type: &str) -> Result<Vec<AlternativeDay>, sqlx::Error> {
    query_as!(AlternativeDay, "select date, COALESCE(description, '') as 'description', slots_start_hour as 'start_hour', slot_duration as 'duration', slot_capacity, created_at
        from alternative_days where type = $1
        order by date desc, created_at", day_type)
        .fetch_all(pool)
        .await
}

async fn delete_alternative_day(state: AppState, date: String) -> HttpResult {
    let Ok(date) = Date::parse(&date, date_formats::ISO_DATE) else {
        return Ok(error_bubble_response("Data selectata e ste invalida"));
    };

    let mut tx = state.write_pool.begin().await?;

    let deleted_reservations = delete_reservations_on_day(tx.as_mut(), date).await?;
    if deleted_reservations != 0 {
        info!("{deleted_reservations} reservation were deleted when deleting alternative day");
    }

    query!("delete from alternative_days where date = $1", date)
        .execute(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(().into_response())
}

async fn alt_day_exists(conn: &mut SqliteConnection, date: Date) -> Result<bool, sqlx::Error> {
    Ok(query!(
        "select exists (select 1 from alternative_days where date = $1) as 'exists!'",
        date
    )
    .fetch_one(conn)
    .await?
    .exists
        != 0)
}

async fn delete_reservations_on_day(
    executor: impl Executor<'_, Database = Sqlite>,
    date: Date,
) -> Result<u64, sqlx::Error> {
    query!("delete from reservations where date = $1", date)
        .execute(executor)
        .await
        .map(|result| result.rows_affected())
}
