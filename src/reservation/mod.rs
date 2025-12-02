mod cancel;
mod check;
mod result;
#[cfg(test)]
mod test;

use crate::model::location::Location;
use crate::model::user::User;
pub use crate::reservation::check::*;
pub use result::*;
use sqlx::{Executor, Sqlite, SqlitePool, SqliteTransaction, query};
use time::{Date, OffsetDateTime};
use tracing::error;

use crate::utils::queries::{get_alt_day_structure_for_day, get_reservations_count_for_slot};
pub use cancel::cancel_reservation;

pub async fn create_reservation(
    pool: &SqlitePool,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult {
    let mut tx = pool.begin().await?;
    let success = is_reservation_possible(
        tx.as_mut(),
        location,
        now,
        user,
        selected_date,
        selected_hour,
    )
    .await?;

    if let ReservationSuccess::Reservation { deletes_guest } = success
        && deletes_guest
    {
        let rows_affected =
            reorder_extra_guest(tx.as_mut(), selected_date, selected_hour, location).await?;
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
    }

    let as_guest = match success {
        ReservationSuccess::Guest => true,
        ReservationSuccess::InWaiting { as_guest } => as_guest,
        _ => false,
    };
    let in_waiting = matches!(success, ReservationSuccess::InWaiting { .. });
    query!(
        "insert into reservations (user_id, location, date, hour, as_guest, in_waiting) values ($1, $2, $3, $4, $5, $6)",
        user.id,
        location.id,
        selected_date,
        selected_hour,
        as_guest,
        in_waiting,
    )
        .execute(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(success)
}

async fn reorder_extra_guest(
    executor: impl Executor<'_, Database = Sqlite>,
    date: Date,
    hour: u8,
    location: &Location,
) -> sqlx::Result<u64> {
    query!(
        "update reservations set in_waiting = true where rowid in
                (select rowid from reservations
                where date = $1 and hour = $2 and location = $3 and 
                    as_guest = true and in_waiting = false and cancelled = false
                order by created_at desc limit 1)",
        date,
        hour,
        location.id
    )
    .execute(executor)
    .await
    .map(|result| result.rows_affected())
}

pub async fn create_referred_guest(
    mut tx: SqliteTransaction<'_>,
    location: &Location,
    date: Date,
    hour: u8,
    user_id: i64,
    special: bool,
    created_for: &str,
) -> sqlx::Result<()> {
    let day_structure = get_alt_day_structure_for_day(tx.as_mut(), date)
        .await
        .unwrap_or_else(|| location.day_structure());

    let capacity = day_structure
        .slot_capacity
        .unwrap_or(location.slot_capacity);

    let slot_reservations =
        get_reservations_count_for_slot(tx.as_mut(), location, date, hour).await?;
    let total_reservations = slot_reservations.member + slot_reservations.guest;

    let in_waiting = total_reservations >= capacity;

    let as_guest = !special;
    query!(
        "insert into reservations (user_id, date, hour, location, created_for, as_guest, in_waiting) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        user_id,
        date,
        hour,
        location.id,
        created_for,
        as_guest,
        in_waiting,
    )
        .execute(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(())
}
