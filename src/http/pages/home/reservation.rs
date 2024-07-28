use std::fmt::{Display, Formatter};
use std::ops::DerefMut;

use sqlx::{query, query_as, Sqlite, Transaction};
use time::{Date, OffsetDateTime};
use crate::http::AppState;
use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::UserUi;
use crate::utils::is_free_day;

#[derive(Debug, PartialEq)]
pub enum ReservationSuccess {
    Reservation,
    Guest,
}

#[derive(Debug, PartialEq)]
pub enum ReservationError {
    AlreadyExists,
    Restriction(String),
    SlotFull,
    DatabaseError(String),
    NoMoreReservation,
    Other(String),
}

impl From<sqlx::Error> for ReservationError {
    fn from(value: sqlx::Error) -> Self {
        ReservationError::DatabaseError(value.to_string())
    }
}

impl Display for ReservationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReservationError::AlreadyExists => write!(f, "Ai făcut deja o astfel de rezervare."),
            ReservationError::Restriction(message) => write!(f, "{}", message),
            ReservationError::SlotFull => write!(
                f,
                "S-a atins numărul maxim de rezervări pentru intervalul orar."
            ),
            ReservationError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ReservationError::NoMoreReservation => {
                write!(f, "Ți-ai epuizat rezervările pe săptămâna aceasta")
            }
            ReservationError::Other(message) => write!(f, "{}", message),
        }
    }
}

fn check_parameters_validity(
    location: &Location,
    now: OffsetDateTime,
    is_free_day: bool,
    selected_date: Date,
    selected_hour: u8,
) -> Result<(), ReservationError> {
    let now_date = now.date();
    let now_hour = now.time().hour() as u8;

    if selected_date < now_date || (selected_date == now_date && selected_hour <= now_hour) {
        return Err(ReservationError::Other(
            "Nu poți face o rezervare în trecut".to_string(),
        ));
    }

    let hour_structure = if is_free_day {
        location.get_alt_hour_structure()
    } else {
        location.get_hour_structure()
    };

    if !hour_structure.is_hour_valid(selected_hour) {
        return Err(ReservationError::Other(
            "Ora pentru rezervare nu este validă".to_string(),
        ));
    }

    if !is_free_day && now_hour == selected_hour - 1 {
        return Err(ReservationError::Other(
            "Rezervările se fac cu cel putin o oră înainte".to_string(),
        ));
    }

    Ok(())
}

async fn check_other_errors<'a>(
    tx: &mut Transaction<'a, Sqlite>,
    location: &Location,
    user: &UserUi,
    selected_date: Date,
    selected_hour: u8,
) -> Result<(), ReservationError> {
    // Check if it already exists
    let reservation_already_exists = query!(
        "select exists(select true from reservations where location = $1 and date = $2 and hour = $3 and user_id = $4) as 'exists!'",
        location.id,
        selected_date,
        selected_hour,
        user.id
    )
        .fetch_one(tx.deref_mut())
        .await
        .map_err(ReservationError::from)?
        .exists;

    if reservation_already_exists == 1 {
        return Err(ReservationError::AlreadyExists);
    }

    let guest_already_exists = query!(
        "select exists(select true from guests where location = $1 and date = $2 and hour = $3 and created_by = $4) as 'exists!'",
        location.id,
        selected_date,
        selected_hour,
        user.id
    )
        .fetch_one(tx.deref_mut())
        .await
        .map_err(ReservationError::from)?
        .exists;

    if guest_already_exists == 1 {
        return Err(ReservationError::AlreadyExists);
    }

    let restriction = query!(
        "select message from reservations_restrictions where location = $1 and date = $2 and (hour = $3 or hour is null)",
        location.id,
        selected_date,
        selected_hour
    )
        .fetch_optional(tx.deref_mut())
        .await
        .map_err(ReservationError::from)?;

    // Check if there is a restriction
    if let Some(restriction) = restriction {
        return Err(ReservationError::Restriction(restriction.message));
    }

    Ok(())
}

pub async fn create_reservation(
    state: &AppState,
    now: OffsetDateTime,
    user: &UserUi,
    selected_date: Date,
    selected_hour: u8,
) -> Result<ReservationSuccess, ReservationError> {
    let is_free_day = is_free_day(&state.pool, &selected_date).await;

    check_parameters_validity(
        &state.location,
        now,
        is_free_day,
        selected_date,
        selected_hour,
    )?;

    let mut tx = state.pool.begin().await.expect("Database error");
    check_other_errors(&mut tx, &state.location, user, selected_date, selected_hour).await?;

    let role = query_as!(
        UserRole,
        "select * from user_roles where name = $1",
        user.role
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?;

    let fixed_reservations_count = query!(r#"select(
    (select count(*) from reservations where location = $1 and date = $2 and hour = $3 and cancelled = false) +
    (select count(*) from special_guests where location = $1 and date = $2 and hour = $3)) as 'count!'"#,
        state.location.id, selected_date, selected_hour)
        .fetch_one(&mut *tx)
        .await
        .map_err(ReservationError::from)?
        .count as i64;

    if fixed_reservations_count >= state.location.slot_capacity {
        return Err(ReservationError::SlotFull);
    }

    let guest_reservations_count =
        query!(
        "select count(*) as 'count!' from guests where location = $1 and date = $2 and hour = $3",
        state.location.id, selected_date, selected_hour)
        .fetch_one(&mut *tx)
        .await
        .map_err(ReservationError::from)?
        .count as i64;

    let total_count = fixed_reservations_count + guest_reservations_count;
    if role.max_reservations == 0 && total_count >= state.location.slot_capacity {
        return Err(ReservationError::SlotFull);
    }

    let user_reservations_count = query!(
        r#"select count(*) as 'count!' from reservations 
            where user_id = $1 and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2) and
            strftime('%w', date) != 0 and strftime('%w', date) != 6"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count as i64;

    let user_reservations_as_guest_count = query!(
        r#"select count(*) as 'count!' from guests
            where created_by = $1 and
            strftime('%Y%W', date) = strftime('%Y%W', $2) and
            strftime('%w', date) != 0 and strftime('%w', date) != 6"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count as i64;

    // Think of all cases
    if user_reservations_count >= role.max_reservations && !is_free_day {
        if user_reservations_as_guest_count >= role.max_guest_reservations && !is_free_day {
            return Err(ReservationError::NoMoreReservation);
        }

        query!(
            "insert into guests (location, date, hour, created_by) values ($1, $2, $3, $4)",
            state.location.id,
            selected_date,
            selected_hour,
            user.id
        )
        .execute(&mut *tx)
        .await
        .map_err(ReservationError::from)?;

        tx.commit().await.map_err(ReservationError::from)?;
        return Ok(ReservationSuccess::Guest);
    }

    query!(
        "insert into reservations (user_id, location, date, hour) values ($1, $2, $3, $4)",
        user.id,
        state.location.id,
        selected_date,
        selected_hour
    )
    .execute(&mut *tx)
    .await
    .map_err(ReservationError::from)?;

    if total_count >= state.location.slot_capacity {
        query!("delete from guests where rowid in (select rowid from guests where date = $1 and hour = $2 order by created_at desc limit 1)",
            selected_date, selected_hour)
            .execute(&mut *tx)
            .await
            .map_err(ReservationError::from)?;
    }

    tx.commit().await.map_err(ReservationError::from)?;

    Ok(ReservationSuccess::Reservation)
}

#[cfg(test)]
mod test {
    use sqlx::SqlitePool;
    use super::*;

    async fn setup(
        pool: SqlitePool,
        user_max_reservations: u8,
        user_max_guest_reservations: u8,
    ) -> (AppState, UserUi, UserUi) {
        sqlx::query!(
            r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, FALSE);
        insert into users (id, email, name, password_hash, role_id, has_key)
        VALUES (1000, 'test@test.com', 'Test', '', 100, FALSE),
        (2000, 'hello@test.com', 'Test', '', 100, FALSE);

        insert into locations (name, slot_capacity, slots_start_hour, slot_duration, slots_per_day, alt_slots_start_hour, alt_slot_duration, alt_slots_per_day)
        VALUES ('test_location', 1, 18, 2, 2, 10, 3, 4);
        "#, user_max_reservations, user_max_guest_reservations
        ).execute(&pool).await.unwrap();

        let state = AppState {
            location: query_as!(
                Location,
                "select * from locations where name = 'test_location'"
            )
            .fetch_one(&pool)
            .await
            .expect("No locations found"),
            pool,
        };

        let user1 = query_as!(UserUi, "select * from users_with_role where id = 1000")
            .fetch_one(&state.pool)
            .await
            .unwrap();

        let user2 = query_as!(UserUi, "select * from users_with_role where id = 2000")
            .fetch_one(&state.pool)
            .await
            .unwrap();

        (state, user1, user2)
    }

    #[sqlx::test]
    async fn no_guest(pool: SqlitePool) {
        let (state, user_1, user_2) = setup(pool, 2, 0).await;

        let date_str = "11.07.2024";
        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = Date::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 18).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        assert_eq!(
            create_reservation(&state, now, &user_2, first_date, 18).await,
            Err(ReservationError::SlotFull)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 20).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, second_date, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
        assert_eq!(
            create_reservation(&state, now, &user_1, second_date, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn with_guest(pool: SqlitePool) {
        let (state, user, _) = setup(pool, 1, 2).await;

        let date_str = "11.07.2024";
        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = Date::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        // As guest
        assert_eq!(
            create_reservation(&state, now, &user, first_date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&state, now, &user, second_date, 18).await,
            Ok(ReservationSuccess::Guest)
        );
        assert_eq!(
            create_reservation(&state, now, &user, second_date, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn only_guest(pool: SqlitePool) {
        let (state, user_1, user_2) = setup(pool, 0, 2).await;

        let date_str = "11.07.2024";
        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = Date::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 18).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&state, now, &user_2, first_date, 18).await,
            Err(ReservationError::SlotFull)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, first_date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, second_date, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn too_late(pool: SqlitePool) {
        let (state, user, _) = setup(pool, 1, 0).await;

        let date_str = "11.07.2024";
        let now_good = OffsetDateTime::parse_from_str("11.07.2024 16:59", "%d.%m.%Y %H:%M").unwrap();
        let now_too_late =
            OffsetDateTime::parse_from_str("11.07.2024 17:00", "%d.%m.%Y %H:%M").unwrap();
        let date = Date::parse_from_str(date_str, "%d.%m.%Y").unwrap();

        assert_eq!(
            create_reservation(&state, now_good, &user, date, 18).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now_too_late, &user, date, 18).await,
            Err(ReservationError::Other(
                "Rezervările se fac cu cel putin o oră înainte".to_string()
            ))
        );
    }

    #[sqlx::test]
    async fn replace_guest(pool: SqlitePool) {
        let (state, user_1, user_2) = setup(pool, 1, 1).await;

        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let date = Date::parse_from_str("11.07.2024", "%d.%m.%Y").unwrap();

        assert_eq!(
            create_reservation(&state, now, &user_1, date, 18).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user_1, date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        // It should replace the guest
        assert_eq!(
            create_reservation(&state, now, &user_2, date, 20).await,
            Ok(ReservationSuccess::Reservation)
        );
    }

    #[sqlx::test]
    async fn free_days(pool: SqlitePool) {
        let (state, user, _) = setup(pool, 1, 1).await;

        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let date = Date::parse_from_str("11.07.2024", "%d.%m.%Y").unwrap();
        let weekend = Date::parse_from_str("13.07.2024", "%d.%m.%Y").unwrap();

        assert_eq!(
            create_reservation(&state, now, &user, date, 18).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user, date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&state, now, &user, weekend, 10).await,
            Ok(ReservationSuccess::Reservation)
        );

        assert_eq!(
            create_reservation(&state, now, &user, weekend, 13).await,
            Ok(ReservationSuccess::Reservation)
        );
    }

    #[sqlx::test]
    async fn restrictions(pool: SqlitePool) {
        let (state, user, _) = setup(pool, 1, 1).await;

        query!("insert into reservations_restrictions (message, location, date, hour) values ('res1', $1, '2024-07-11', NULL), ('res2', $1, '2024-07-12', 18)", state.location.id)
            .execute(&state.pool)
            .await
            .unwrap();

        let now = OffsetDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let date_1 = Date::parse_from_str("11.07.2024", "%d.%m.%Y").unwrap();
        let date_2 = Date::parse_from_str("12.07.2024", "%d.%m.%Y").unwrap();

        assert_eq!(
            create_reservation(&state, now, &user, date_1, 18).await,
            Err(ReservationError::Restriction("res1".to_string()))
        );

        assert_eq!(
            create_reservation(&state, now, &user, date_1, 20).await,
            Err(ReservationError::Restriction("res1".to_string()))
        );

        assert_eq!(
            create_reservation(&state, now, &user, date_2, 18).await,
            Err(ReservationError::Restriction("res2".to_string()))
        );

        assert_eq!(
            create_reservation(&state, now, &user, date_2, 20).await,
            Ok(ReservationSuccess::Reservation)
        );
    }
}
