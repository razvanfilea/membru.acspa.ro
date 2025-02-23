use crate::model::day_structure::DayStructure;
use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::reservation::{ReservationError, ReservationResult, ReservationSuccess};
use crate::utils::queries::{
    get_alt_day_structure_for_day, get_current_reservations_count,
    get_user_weeks_reservations_count,
};
use sqlx::{SqliteConnection, query, query_as};
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
    tx: &mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult {
    let day_structure = get_alt_day_structure_for_day(&mut *tx, selected_date)
        .await
        .unwrap_or_else(|| location.day_structure());

    check_parameters_validity(now, &day_structure, selected_date, selected_hour)?;

    check_reservation_already_exists(&mut *tx, location, user, selected_date, selected_hour)
        .await?;

    check_restriction(&mut *tx, location, selected_date, selected_hour).await?;

    let role = query_as!(
        UserRole,
        "select * from user_roles where id = $1",
        user.role_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let current_reservations =
        get_current_reservations_count(&mut *tx, location, selected_date, selected_hour).await?;
    let total_reservations = current_reservations.member + current_reservations.guest;

    let capacity = day_structure
        .slot_capacity
        .unwrap_or(location.slot_capacity);

    let user_reservations_count =
        get_user_weeks_reservations_count(&mut *tx, user, selected_date).await?;

    // Attempt to create a normal reservation
    if user_reservations_count.member < role.reservations {
        return Ok(if total_reservations < capacity {
            ReservationSuccess::Reservation {
                deletes_guest: false,
            }
        } else if current_reservations.member < capacity {
            ReservationSuccess::Reservation {
                deletes_guest: true,
            }
        } else {
            ReservationSuccess::InWaiting { as_guest: false }
        });
    }

    // Otherwise try to create a guest reservation
    if user_reservations_count.guest < role.guest_reservations {
        return Ok(if total_reservations < capacity {
            ReservationSuccess::Guest
        } else {
            ReservationSuccess::InWaiting { as_guest: true }
        });
    }

    Err(ReservationError::NoMoreReservations)
}
