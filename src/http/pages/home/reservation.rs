use crate::http::AppState;
use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::UserUi;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Utc, Weekday};
use sqlx::{query, query_as, Sqlite, SqlitePool, Transaction};
use std::fmt::{Display, Formatter};
use std::ops::DerefMut;

pub async fn is_free_day(pool: &SqlitePool, date: &NaiveDate) -> bool {
    let exists_in_table = async {
        query!(
            "select exists(select true from free_days where date = $1) as 'exists!'",
            date
        )
        .fetch_one(pool)
        .await
        .expect("Database error")
        .exists
            != 0
    };

    date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun || exists_in_table.await
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
            ReservationError::DatabaseError(e) => write!(f, "{}", e),
            ReservationError::NoMoreReservation => {
                write!(f, "Ți-ai epuizat rezervările pe săptămâna aceasta")
            }
            ReservationError::Other(message) => write!(f, "{}", message),
        }
    }
}

fn check_parameters_valid(
    now: NaiveDateTime,
    is_free_day: bool,
    selected_date: NaiveDate,
    selected_hour: u8,
) -> Result<(), ReservationError> {
    let now_date = now.date();
    let now_hour = now.time().hour() as u8;

    if selected_date < now_date || (selected_date == now_date && selected_hour <= now_hour) {
        return Err(ReservationError::Other(
            "Nu poți face o rezervare în trecut".to_string(),
        ));
    }

    if !is_free_day && now_hour == selected_hour - 1 {
        return Err(ReservationError::Other(
            "Rezervările se fac cu cel putin o oră înainte".to_string(),
        ));
    }

    Ok(())
}

async fn check_other<'a>(
    tx: &mut Transaction<'a, Sqlite>,
    location: &Location,
    user: &UserUi,
    selected_date: NaiveDate,
    selected_hour: u8,
) -> Result<(), ReservationError> {
    // Check if it already exists
    let reservation_already_exists = query!(
        "select exists(select true from reservations where location = $1 and date = $2 and hour = $3 and user_id = $4 and cancelled = false) as 'exists!'",
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
        "select exists(select true from guests where location = $1 and date = $2 and hour = $3 and created_by = $4 and special = false) as 'exists!'",
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
        "select message from reservations_restrictions where location = $1 and date = $2 and hour = $3",
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
    now: NaiveDateTime,
    user: &UserUi,
    selected_date: NaiveDate,
    selected_hour: u8,
) -> Result<String, ReservationError> {
    let is_free_day = is_free_day(&state.pool, &selected_date).await;

    check_parameters_valid(now, is_free_day, selected_date, selected_hour)?;

    let mut tx = state.pool.begin().await.expect("Database error");
    check_other(&mut tx, &state.location, user, selected_date, selected_hour).await?;

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
    (select count(*) from guests where location = $1 and date = $2 and hour = $3 and special = true)) as 'count!'"#,
        state.location.id, selected_date, selected_hour)
        .fetch_one(&mut *tx)
        .await
        .map_err(ReservationError::from)?
        .count as i64;

    if fixed_reservations_count > state.location.slot_capacity {
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
    if user_reservations_count >= role.reservations && !is_free_day {
        if user_reservations_as_guest_count >= role.as_guest && !is_free_day {
            return Err(ReservationError::NoMoreReservation);
        }

        query!("insert into guests (guest_name, location, date, hour, created_by) values ($1, $2, $3, $4, $5)",
                user.name, state.location.id, selected_date, selected_hour, user.id)
            .execute(&mut *tx)
            .await
            .map_err(ReservationError::from)?;

        tx.commit().await.map_err(ReservationError::from)?;
        // TODO Review message for all cases
        Ok(format!("Ai fost înscris ca invitat de la ora {}. Nu poți face decât {} rezervări pe săptămână (luni - vineri) ca și {}",
                   selected_hour, role.reservations, role.name))
    } else {
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

        tx.commit().await.map_err(ReservationError::from)?;
        Ok(format!(
            "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
            selected_date.format("%d.%m.%Y")
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    async fn setup(
        pool: SqlitePool,
        max_reservations: u8,
        max_guest_reservations: u8,
    ) -> (AppState, UserUi) {
        sqlx::query!(
            r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, FALSE);
        insert into users (id, email, name, password_hash, role_id, has_key)
        VALUES (1000, 'test@test.com', 'Test', '', 100, FALSE);

        insert into locations (name, slot_capacity, slots_start_hour, slot_duration, slots_per_day, alt_slots_start_hour, alt_slot_duration, alt_slots_per_day)
        VALUES ('test_location', 2, 18, 2, 2, 10, 3, 4);
        "#, max_reservations, max_guest_reservations
        ).execute(&pool).await.unwrap();

        let state = AppState::new(pool).await;

        let user = query_as!(UserUi, "select * from users_with_role where id = 1000")
            .fetch_one(&state.pool)
            .await
            .unwrap();

        (state, user)
    }

    #[sqlx::test]
    async fn no_guest(pool: SqlitePool) {
        let (state, user) = setup(pool, 2, 0).await;

        let date_str = "11.07.2024";
        let now = NaiveDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Ok(format!(
                "Ai rezervare pe data de <b>{date_str}</b> de la ora <b>18:00</b>"
            ))
        );

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 20).await,
            Ok(format!(
                "Ai rezervare pe data de <b>{date_str}</b> de la ora <b>20:00</b>"
            ))
        );

        assert_eq!(
            create_reservation(&state, now, &user, second_date, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
        assert_eq!(
            create_reservation(&state, now, &user, second_date, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn with_guest(pool: SqlitePool) {
        let (state, user) = setup(pool, 1, 2).await;

        let date_str = "11.07.2024";
        let now = NaiveDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Ok(format!(
                "Ai rezervare pe data de <b>{date_str}</b> de la ora <b>18:00</b>"
            ))
        );

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        // As guest
        assert!(create_reservation(&state, now, &user, first_date, 20)
            .await
            .is_ok());

        assert!(create_reservation(&state, now, &user, second_date, 18)
            .await
            .is_ok());
        assert_eq!(
            create_reservation(&state, now, &user, second_date, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn only_guest(pool: SqlitePool) {
        let (state, user) = setup(pool, 0, 2).await;

        let date_str = "11.07.2024";
        let now = NaiveDateTime::parse_from_str("11.07.2024 10:00", "%d.%m.%Y %H:%M").unwrap();
        let first_date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").unwrap();
        let second_date = first_date.with_day(12).unwrap();

        assert!(create_reservation(&state, now, &user, first_date, 18)
            .await
            .is_ok());

        assert_eq!(
            create_reservation(&state, now, &user, first_date, 18).await,
            Err(ReservationError::AlreadyExists)
        );

        assert!(create_reservation(&state, now, &user, first_date, 20)
            .await
            .is_ok());

        assert_eq!(
            create_reservation(&state, now, &user, second_date, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
    }


    #[sqlx::test]
    async fn too_late(pool: SqlitePool) {
        let (state, user) = setup(pool, 0, 2).await;

        let date_str = "11.07.2024";
        let now_good = NaiveDateTime::parse_from_str("11.07.2024 16:59", "%d.%m.%Y %H:%M").unwrap();
        let now_too_late = NaiveDateTime::parse_from_str("11.07.2024 17:00", "%d.%m.%Y %H:%M").unwrap();
        let date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").unwrap();

        assert!(create_reservation(&state, now_good, &user, date, 18)
            .await
            .is_ok());

        assert_eq!(
            create_reservation(&state, now_too_late, &user, date, 18).await,
            Err(ReservationError::Other("Rezervările se fac cu cel putin o oră înainte".to_string()))
        );
    }
}
