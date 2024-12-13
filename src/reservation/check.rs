use crate::model::hour_structure::HourStructure;
use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::reservation::{ReservationError, ReservationResult, ReservationSuccess};
use crate::utils::queries::get_alt_hour_structure_for_day;
use sqlx::{query, query_as, SqliteConnection};
use time::{Date, OffsetDateTime};

fn check_parameters_validity(
    now: OffsetDateTime,
    hour_structure: &HourStructure,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult<()> {
    let now_date = now.date();
    let now_hour = now.time().hour();

    if selected_date < now_date || (selected_date == now_date && selected_hour <= now_hour) {
        return Err(ReservationError::Other(
            "Nu poți face o rezervare în trecut",
        ));
    }

    if !hour_structure.is_hour_valid(selected_hour) {
        return Err(ReservationError::Other(
            "Ora pentru rezervare nu este validă",
        ));
    }

    if selected_date == now_date && now_hour == selected_hour - 1 {
        return Err(ReservationError::Other(
            "Rezervările se fac cu cel putin o oră înainte",
        ));
    }

    Ok(())
}

async fn check_reservation_already_exists(
    tx: &mut SqliteConnection,
    location: &Location,
    user: &User,
    date: Date,
    hour: u8,
) -> ReservationResult<()> {
    let reservation_already_exists = query!(
        "select cancelled from reservations where
        location = $1 and date = $2 and hour = $3 and user_id = $4 and created_for is null",
        location.id,
        date,
        hour,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(reservation) = reservation_already_exists {
        return Err(ReservationError::AlreadyExists {
            cancelled: reservation.cancelled,
        });
    }

    Ok(())
}

async fn check_restriction(
    tx: &mut SqliteConnection,
    location: &Location,
    date: Date,
    hour: u8,
) -> ReservationResult<()> {
    let restriction = query!(
        "select message from restrictions where location = $1 and date = $2 and (hour = $3 or hour is null)",
        location.id,
        date,
        hour
    )
        .fetch_optional(&mut *tx)
        .await?;

    // Check if there is a restriction
    if let Some(restriction) = restriction {
        return Err(ReservationError::Restriction(restriction.message));
    }

    Ok(())
}

pub async fn is_reservation_possible(
    tx: &'_ mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult {
    let hour_structure = get_alt_hour_structure_for_day(&mut *tx, selected_date)
        .await
        .unwrap_or_else(|| location.hour_structure());

    check_parameters_validity(now, &hour_structure, selected_date, selected_hour)?;

    check_reservation_already_exists(&mut *tx, location, user, selected_date, selected_hour)
        .await?;

    check_restriction(&mut *tx, location, selected_date, selected_hour).await?;

    let role = query_as!(
        UserRole,
        "select * from user_roles where name = $1",
        user.role
    )
    .fetch_one(&mut *tx)
    .await?;

    let regular_reservations_count = query!(
        "select count(*) as 'count!' from reservations where
        location = $1 and date = $2 and hour = $3 and as_guest = false and cancelled = false and in_waiting = false",
        location.id,
        selected_date,
        selected_hour
    )
        .fetch_one(&mut *tx)
        .await?
        .count;

    let guest_reservations_count = query!(
        "select count(*) as 'count!' from reservations where
        location = $1 and date = $2 and hour = $3 and as_guest = true and cancelled = false and in_waiting = false",
        location.id,
        selected_date,
        selected_hour
    )
        .fetch_one(&mut *tx)
        .await?
        .count;

    let total_reservations = regular_reservations_count + guest_reservations_count;

    let regular_user_reservations_count = query!(
        r#"select count(*) as 'count!' from reservations
            where user_id = $1 and as_guest = false and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2)"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await?
    .count;

    let guest_user_reservations_count = query!(
        r#"select count(*) as 'count!' from reservations
            where user_id = $1 and as_guest = true and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2)"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await?
    .count;

    if regular_user_reservations_count < role.reservations {
        return Ok(if total_reservations < location.slot_capacity {
            ReservationSuccess::Reservation {
                deletes_guest: false,
            }
        } else if regular_reservations_count < location.slot_capacity {
            ReservationSuccess::Reservation {
                deletes_guest: true,
            }
        } else {
            ReservationSuccess::InWaiting { as_guest: false }
        });
    }

    if guest_user_reservations_count < role.guest_reservations {
        return Ok(if total_reservations < location.slot_capacity {
            ReservationSuccess::Guest
        } else {
            ReservationSuccess::InWaiting { as_guest: true }
        });
    }

    Err(ReservationError::NoMoreReservation)
}
