use crate::model::location::Location;
use sqlx::{SqliteTransaction, query, query_scalar};
use time::Date;

pub async fn cancel_reservation(
    mut tx: SqliteTransaction<'_>,
    location: &Location,
    date: Date,
    hour: u8,
    user_id: i64,
    created_for: Option<&str>,
) -> sqlx::Result<bool> {
    let rows = if let Some(created_for) = created_for {
        query!("delete from reservations where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for = $5",
            date, hour, user_id, location.id, created_for)
            .execute(tx.as_mut())
            .await?
    } else {
        query!("update reservations set cancelled = true
        where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for is null",
            date, hour, user_id, location.id)
            .execute(tx.as_mut())
            .await?
    }.rows_affected();

    if rows != 1 {
        return Ok(false);
    }

    let count = query_scalar!(
        "select count(*) as 'count!' from reservations where
            date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = false",
        date,
        hour,
        location.id
    )
    .fetch_one(tx.as_mut())
    .await?;

    if count < location.slot_capacity {
        query!(
            "update reservations set in_waiting = false where rowid =
                (select rowid from reservations where
                    date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = true
                    order by as_guest, created_at limit 1)",
            date, hour, location.id)
            .execute(tx.as_mut())
            .await?;
    }

    tx.commit().await?;

    Ok(true)
}
