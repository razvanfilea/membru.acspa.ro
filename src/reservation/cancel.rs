use crate::model::day_structure::DayStructure;
use crate::model::location::Location;
use sqlx::{SqliteConnection, SqliteTransaction, query, query_scalar};
use time::{Date, OffsetDateTime};
use tracing::info;

/// A reservation can be cancelled until its slot starts, `start_minute` is the day's `slots_start_minute`
pub fn is_cancellable_at(
    date: Date,
    hour: u8,
    start_minute: Option<i64>,
    now: OffsetDateTime,
) -> bool {
    let today = now.date();
    let slot_start = i64::from(hour) * 60 + start_minute.unwrap_or(0);
    let current = i64::from(now.hour()) * 60 + i64::from(now.minute());
    date > today || (date == today && slot_start > current)
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

/// Same as [`cancel_reservation`], but runs inside the caller's transaction without committing it
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

/// Removes the reservations of a user that haven't started yet, used when the user is deleted.
/// Each one is cancelled first, so the waiting list moves up and the history records who did it,
/// then the rows are deleted so they don't show up as cancelled.
/// Past and in-progress reservations are kept. Returns how many were cancelled.
pub async fn remove_upcoming_reservations(
    tx: &mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user_id: i64,
    removed_by: i64,
) -> sqlx::Result<usize> {
    let today = now.date();
    let current_minutes = i64::from(now.hour()) * 60 + i64::from(now.minute());

    // Same boundary as `is_cancellable_at`: a slot is upcoming until it starts
    let upcoming = query!(
        r#"select date as "date: Date", hour as "hour: u8", created_for from reservations
        where user_id = $1 and location = $2 and cancelled = false
            and (date > $3 or (date = $3 and hour * 60 + ifnull(
                (select slots_start_minute from alternative_days a where a.date = reservations.date), 0
            ) > $4))"#,
        user_id,
        location.id,
        today,
        current_minutes
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
            removed_by,
        )
        .await?;
    }

    // Cancelled rows are skipped by the history delete trigger, so each keeps just its cancellation
    query!(
        "delete from reservations
        where user_id = $1 and location = $2 and cancelled = true
            and (date > $3 or (date = $3 and hour * 60 + ifnull(
                (select slots_start_minute from alternative_days a where a.date = reservations.date), 0
            ) > $4))",
        user_id,
        location.id,
        today,
        current_minutes
    )
    .execute(&mut *tx)
    .await?;

    Ok(upcoming.len())
}
