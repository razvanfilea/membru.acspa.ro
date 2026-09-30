mod cancel;
mod check;
mod result;
#[cfg(test)]
mod test;

use crate::model::location::Location;
use crate::model::user::User;
pub use crate::reservation::check::*;
pub use result::*;
use sqlx::{SqliteExecutor, SqlitePool, query, query_as};
use time::{Date, Duration, OffsetDateTime};
use tracing::{error, info};

pub use cancel::{cancel_reservation, is_cancellable_at, remove_upcoming_reservations};

pub const HISTORY_RETENTION_DAYS: i64 = 90;

/// Oldest reservation date whose changes can still be in `reservation_history`
pub fn history_retention_start(today: Date) -> Date {
    today - Duration::days(HISTORY_RETENTION_DAYS)
}

pub fn is_within_history_retention(date: Date, today: Date) -> bool {
    date >= history_retention_start(today)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Referral<'a> {
    pub is_special: bool,
    pub created_for: &'a str,
}

pub async fn create_reservation(
    pool: &SqlitePool,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
    referral: Option<Referral<'_>>,
) -> ReservationResult {
    let result = try_create_reservation(
        pool,
        location,
        now,
        user,
        selected_date,
        selected_hour,
        referral,
    )
    .await;

    info!(
        "Reservation request by user {} on {selected_date} hour {selected_hour} (created_for: {:?}): {result:?}",
        user.id,
        referral.map(|r| r.created_for)
    );

    result
}

async fn try_create_reservation(
    pool: &SqlitePool,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
    referral: Option<Referral<'_>>,
) -> ReservationResult {
    let mut tx = pool.begin().await?;
    let success = is_reservation_possible(
        tx.as_mut(),
        location,
        now,
        user,
        selected_date,
        selected_hour,
        referral,
    )
    .await?;

    if let ReservationSuccess::Reservation { deletes_guest } = success
        && deletes_guest
    {
        let demoted =
            reorder_extra_guest(tx.as_mut(), selected_date, selected_hour, location).await?;
        let rows_affected = demoted.len();
        if rows_affected > 1 {
            error!("Updated more than one guest reservation");
            return Err(ReservationError::DatabaseError(
                "Updated more than one guest reservation".to_string(),
            ));
        }

        if rows_affected == 0 {
            return Err(ReservationError::DatabaseError(
                "Nu s-a putut șterge un invitat".to_string(),
            ));
        }

        for guest in demoted {
            info!(
                "Guest reservation of user {} (created_for: {:?}) on {selected_date} hour {selected_hour} moved to waiting list",
                guest.user_id, guest.created_for
            );
        }
    }

    let as_guest = match success {
        ReservationSuccess::Guest => true,
        ReservationSuccess::InWaiting { as_guest } => as_guest,
        _ => false,
    };
    let in_waiting = matches!(success, ReservationSuccess::InWaiting { .. });
    let created_for = referral.map(|r| r.created_for);
    query!(
        "insert into reservations (user_id, location, date, hour, as_guest, in_waiting, created_for) values ($1, $2, $3, $4, $5, $6, $7)",
        user.id,
        location.id,
        selected_date,
        selected_hour,
        as_guest,
        in_waiting,
        created_for
    )
        .execute(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(success)
}

pub async fn delete_expired_waiting(
    executor: impl SqliteExecutor<'_>,
    current_date: Date,
    current_hour: u8,
) -> sqlx::Result<u64> {
    query!(
        "delete from reservations where in_waiting = true and (date < $1 or (date == $1 and hour <= $2))",
        current_date,
        current_hour
    )
    .execute(executor)
    .await
    .map(|result| result.rows_affected())
}

struct MovedReservation {
    user_id: i64,
    created_for: Option<String>,
}

async fn reorder_extra_guest(
    executor: impl SqliteExecutor<'_>,
    date: Date,
    hour: u8,
    location: &Location,
) -> sqlx::Result<Vec<MovedReservation>> {
    query_as!(
        MovedReservation,
        "update reservations set in_waiting = true where rowid in
                (select rowid from reservations
                where date = $1 and hour = $2 and location = $3 and 
                    as_guest = true and in_waiting = false and cancelled = false
                order by created_at desc limit 1)
        returning user_id as \"user_id!\", created_for",
        date,
        hour,
        location.id
    )
    .fetch_all(executor)
    .await
}
