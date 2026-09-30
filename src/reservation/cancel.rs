use crate::model::day_structure::DayStructure;
use crate::model::location::Location;
use sqlx::{SqliteConnection, SqliteTransaction, query, query_scalar};
use time::{Date, OffsetDateTime};
use tracing::info;

/// A reservation can be cancelled until the hour its slot starts in has passed
pub fn is_cancellable_at(date: Date, hour: u8, now: OffsetDateTime) -> bool {
    let today = now.date();
    date > today || (date == today && hour >= now.hour())
}

pub async fn cancel_reservation(
    mut tx: SqliteTransaction<'_>,
    location: &Location,
    date: Date,
    hour: u8,
    user_id: i64,
    created_for: Option<&str>,
    cancelled_by: i64,
) -> sqlx::Result<bool> {
    let cancelled = cancel_reservation_in(
        &mut tx,
        location,
        date,
        hour,
        user_id,
        created_for,
        cancelled_by,
    )
    .await?;

    if cancelled {
        tx.commit().await?;
    }

    Ok(cancelled)
}

/// Same as [cancel_reservation], but runs inside the caller's transaction without committing it
async fn cancel_reservation_in(
    tx: &mut SqliteConnection,
    location: &Location,
    date: Date,
    hour: u8,
    user_id: i64,
    created_for: Option<&str>,
    cancelled_by: i64,
) -> sqlx::Result<bool> {
    let rows = if let Some(created_for) = created_for {
        // Stamp the actor first so the delete trigger can record who removed the guest
        query!("update reservations set cancelled_by = $6
        where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for = $5",
            date, hour, user_id, location.id, created_for, cancelled_by)
            .execute(&mut *tx)
            .await?;

        query!("delete from reservations where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for = $5",
            date, hour, user_id, location.id, created_for)
            .execute(&mut *tx)
            .await?
    } else {
        query!("update reservations set cancelled = true, cancelled_by = $5
        where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for is null and cancelled = false",
            date, hour, user_id, location.id, cancelled_by)
            .execute(&mut *tx)
            .await?
    }.rows_affected();

    if rows != 1 {
        return Ok(false);
    }

    let count = query_scalar!(
        "select count(*) from reservations where
            date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = false",
        date,
        hour,
        location.id
    )
    .fetch_one(&mut *tx)
    .await?;

    let day_structure = DayStructure::fetch_or_default(&mut *tx, date, location).await?;

    let effective_capacity = day_structure
        .slot_capacity
        .unwrap_or(location.slot_capacity);

    if count < effective_capacity {
        let promoted = query!(
            "update reservations set in_waiting = false where rowid =
                (select rowid from reservations where
                    date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = true
                    order by as_guest, created_at limit 1)
            returning user_id, created_for",
            date, hour, location.id)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(promoted) = promoted {
            info!(
                "Reservation of user {} (created_for: {:?}) on {date} hour {hour} promoted from waiting list",
                promoted.user_id, promoted.created_for
            );
        }
    }

    Ok(true)
}

/// Cancels the reservations of a user that haven't started yet, promoting from the waiting list as needed.
/// Past and in-progress reservations are kept for history. Returns how many were cancelled.
pub async fn cancel_upcoming_reservations(
    tx: &mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user_id: i64,
    cancelled_by: i64,
) -> sqlx::Result<usize> {
    let today = now.date();
    let current_hour = now.hour();

    let upcoming = query!(
        r#"select date as "date: Date", hour as "hour: u8", created_for from reservations
        where user_id = $1 and location = $2 and cancelled = false
            and (date > $3 or (date = $3 and hour > $4))"#,
        user_id,
        location.id,
        today,
        current_hour
    )
    .fetch_all(&mut *tx)
    .await?;

    for reservation in &upcoming {
        cancel_reservation_in(
            tx,
            location,
            reservation.date,
            reservation.hour,
            user_id,
            reservation.created_for.as_deref(),
            cancelled_by,
        )
        .await?;
    }

    Ok(upcoming.len())
}
