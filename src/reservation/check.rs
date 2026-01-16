use crate::model::day_structure::DayStructure;
use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::model::user_reservation::ReservationsCount;
use crate::reservation::{Referral, ReservationError, ReservationResult, ReservationSuccess};
use sqlx::{SqliteConnection, query};
use time::{Date, OffsetDateTime};

fn check_parameters_validity(
    now: OffsetDateTime,
    day_structure: &DayStructure,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult<()> {
    let now_date = now.date();
    let now_hour = now.time().hour();

    if selected_date < now_date {
        return Err(ReservationError::Other(
            "Nu poți face o rezervare pentru o zi din trecut",
        ));
    }

    if !day_structure.is_hour_valid(selected_hour) {
        return Err(ReservationError::Other(
            "Ora pentru rezervare nu este validă",
        ));
    }

    if selected_date == now_date && now_hour >= selected_hour - 1 {
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
    created_for: Option<&str>,
) -> ReservationResult<()> {
    let reservation_already_exists = query!(
        "select cancelled from reservations where
        location = $1 and date = $2 and hour = $3 and user_id = $4 and (created_for = $5 or ($5 is null and created_for is null))",
        location.id,
        date,
        hour,
        user.id,
        created_for
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
    tx: &mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
    referral: Option<Referral<'_>>,
) -> ReservationResult {
    let day_structure = DayStructure::fetch_or_default(&mut *tx, selected_date, location).await?;

    check_parameters_validity(now, &day_structure, selected_date, selected_hour)?;

    check_reservation_already_exists(
        &mut *tx,
        location,
        user,
        selected_date,
        selected_hour,
        referral.map(|r| r.created_for),
    )
    .await?;

    check_restriction(&mut *tx, location, selected_date, selected_hour).await?;

    let role = UserRole::fetch(&mut *tx, user.role_id).await?;

    let slot_reservations =
        ReservationsCount::fetch_for_slot(&mut *tx, location, selected_date, selected_hour).await?;
    let total_reservations = slot_reservations.member + slot_reservations.guest;

    let capacity = day_structure
        .slot_capacity
        .unwrap_or(location.slot_capacity);

    let user_reservations_count =
        ReservationsCount::fetch_user_week(&mut *tx, user, selected_date).await?;

    // Attempt to create a normal reservation
    if (referral.is_none() && user_reservations_count.member < role.reservations)
        || referral.is_some_and(|r| r.is_special)
    {
        return Ok(if total_reservations < capacity {
            ReservationSuccess::Reservation {
                deletes_guest: false,
            }
        } else if slot_reservations.member < capacity {
            ReservationSuccess::Reservation {
                deletes_guest: true,
            }
        } else {
            ReservationSuccess::InWaiting { as_guest: false }
        });
    }

    // Otherwise try to create a guest reservation
    if (referral.is_none() && user_reservations_count.guest < role.guest_reservations)
        || referral.is_some_and(|r| !r.is_special)
    {
        return Ok(if total_reservations < capacity {
            ReservationSuccess::Guest
        } else {
            ReservationSuccess::InWaiting { as_guest: true }
        });
    }

    Err(ReservationError::NoMoreReservations)
}
