mod cancel;
mod check;
mod result;
#[cfg(test)]
mod test;

use crate::model::location::Location;
use crate::model::user::User;
pub use crate::reservation::check::*;
pub use result::*;
use sqlx::{Executor, Sqlite, SqlitePool, query};
use time::{Date, OffsetDateTime};
use tracing::error;

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

    let as_guest = success == ReservationSuccess::Guest;
    let in_waiting = matches!(success, ReservationSuccess::InWaiting { .. });
    assert!(!(in_waiting && as_guest));
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
