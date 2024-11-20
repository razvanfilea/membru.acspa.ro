use std::fmt::{Display, Formatter};

use crate::model::location::Location;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::utils::is_free_day;
use sqlx::{query, query_as, SqliteConnection, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::error;

#[derive(Debug, PartialEq)]
pub enum ReservationSuccess {
    Reservation { deletes_guest: bool },
    Guest,
    InWaiting,
}

#[derive(Debug, PartialEq)]
pub enum ReservationError {
    AlreadyExists { cancelled: bool },
    Restriction(String),
    SlotFull,
    DatabaseError(String),
    NoMoreReservation,
    Other(&'static str),
}

pub type ReservationResult<T = ReservationSuccess> = Result<T, ReservationError>;

impl From<sqlx::Error> for ReservationError {
    fn from(value: sqlx::Error) -> Self {
        ReservationError::DatabaseError(value.to_string())
    }
}

impl Display for ReservationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReservationError::AlreadyExists { cancelled } => {
                if *cancelled {
                    write!(f, "Nu te poți reînscrie după ce ai anulat o rezervare")
                } else {
                    write!(f, "Ai făcut deja o astfel de rezervare.")
                }
            }
            ReservationError::Restriction(message) => write!(f, "{}", message),
            ReservationError::SlotFull => write!(
                f,
                "S-a atins numărul maxim de rezervări pentru intervalul orar."
            ),
            ReservationError::DatabaseError(e) => write!(
                f,
                "Eroare cu baza de date, trimite te rog un screenshot cu aceasta eroare: {}",
                e
            ),
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
) -> ReservationResult<()> {
    let now_date = now.date();
    let now_hour = now.time().hour();

    if selected_date < now_date || (selected_date == now_date && selected_hour <= now_hour) {
        return Err(ReservationError::Other(
            "Nu poți face o rezervare în trecut",
        ));
    }

    let hour_structure = if is_free_day {
        location.get_alt_hour_structure()
    } else {
        location.get_hour_structure()
    };

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

pub async fn is_reservation_possible(
    tx: &'_ mut SqliteConnection,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult {
    let is_free_day = is_free_day(&mut *tx, selected_date).await;

    check_parameters_validity(location, now, is_free_day, selected_date, selected_hour)?;

    let reservation_already_exists = query!(
        "select cancelled from reservations where
        location = $1 and date = $2 and hour = $3 and user_id = $4 and created_for is null",
        location.id,
        selected_date,
        selected_hour,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ReservationError::from)?;

    if let Some(reservation) = reservation_already_exists {
        return Err(ReservationError::AlreadyExists {
            cancelled: reservation.cancelled,
        });
    }

    let restriction = query!(
        "select message from restrictions where location = $1 and date = $2 and (hour = $3 or hour is null)",
        location.id,
        selected_date,
        selected_hour
    )
        .fetch_optional(&mut *tx)
        .await
        .map_err(ReservationError::from)?;

    // Check if there is a restriction
    if let Some(restriction) = restriction {
        return Err(ReservationError::Restriction(restriction.message));
    }

    let role = query_as!(
        UserRole,
        "select * from user_roles where name = $1",
        user.role
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?;

    let fixed_reservations_count = query!(
        "select count(*) as 'count!' from reservations where
        location = $1 and date = $2 and hour = $3 and as_guest = false and cancelled = false and in_waiting = false",
        location.id,
        selected_date,
        selected_hour
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count;

    let user_reservations_count = query!(
        r#"select count(*) as 'count!' from reservations
            where user_id = $1 and as_guest = false and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2)"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count;

    if fixed_reservations_count >= location.slot_capacity {
        let in_waiting_count = query!(
            "select count(*) as 'count!' from reservations where
            location = $1 and date = $2 and hour = $3 and cancelled = false and in_waiting = true",
            location.id,
            selected_date,
            selected_hour
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(ReservationError::from)?
        .count;

        if user_reservations_count < role.reservations
            && in_waiting_count < location.waiting_capacity
        {
            return Ok(ReservationSuccess::InWaiting);
        }

        return Err(ReservationError::SlotFull);
    }

    let guest_reservations_count = query!(
        "select count(*) as 'count!' from reservations where
        location = $1 and date = $2 and hour = $3 and as_guest = true and in_waiting = false",
        location.id,
        selected_date,
        selected_hour
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count;

    let total_count = fixed_reservations_count + guest_reservations_count;
    if role.reservations == 0 && total_count >= location.slot_capacity {
        return Err(ReservationError::SlotFull);
    }

    let user_reservations_as_guest_count = query!(
        r#"select count(*) as 'count!' from reservations
            where user_id = $1 and as_guest = true and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2)"#,
        user.id,
        selected_date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ReservationError::from)?
    .count;

    if user_reservations_count >= role.reservations {
        if user_reservations_as_guest_count >= role.guest_reservations {
            return Err(ReservationError::NoMoreReservation);
        }

        return Ok(ReservationSuccess::Guest);
    }

    Ok(ReservationSuccess::Reservation {
        deletes_guest: total_count >= location.slot_capacity,
    })
}

pub async fn create_reservation(
    pool: &SqlitePool,
    location: &Location,
    now: OffsetDateTime,
    user: &User,
    selected_date: Date,
    selected_hour: u8,
) -> ReservationResult {
    let mut tx = pool.begin().await.map_err(ReservationError::from)?;
    let success = is_reservation_possible(
        tx.as_mut(),
        location,
        now,
        user,
        selected_date,
        selected_hour,
    )
    .await?;

    if let ReservationSuccess::Reservation { deletes_guest } = success {
        if deletes_guest {
            let deleted_guests = query!(
                "delete from reservations where rowid in
                (select rowid from reservations
                where date = $1 and hour = $2 and location = $3 and as_guest = true
                order by created_at desc limit 1)",
                selected_date,
                selected_hour,
                location.id
            )
            .execute(tx.as_mut())
            .await
            .map_err(ReservationError::from)?
            .rows_affected();

            if deleted_guests > 1 {
                error!("Deleted more than one guest reservation");
                return Err(ReservationError::DatabaseError(
                    "Deleted more than one guest reservation".to_string(),
                ));
            }

            if deleted_guests == 0 {
                return Err(ReservationError::SlotFull);
            }
        }
    }

    // sanity check
    {
        let total_reservation_in_slot = query!(
            "select count(*) as 'count!' from reservations where
            location = $1 and date = $2 and hour = $3 and cancelled = false and in_waiting = false",
            location.id,
            selected_date,
            selected_hour
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(ReservationError::from)?
        .count;

        if total_reservation_in_slot >= location.slot_capacity && success != ReservationSuccess::InWaiting {
            return Err(
                ReservationError::DatabaseError(format!("A apărut o eroare la însciere pentru data {selected_date} ora {selected_hour} ca {:?}, te rog trimite un screenshot cu aceasta eroare unui administrator.", success)));
        }
    }

    let as_guest = success == ReservationSuccess::Guest;
    let in_waiting = success == ReservationSuccess::InWaiting;
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
        .await
        .map_err(ReservationError::from)?;

    tx.commit().await.map_err(ReservationError::from)?;

    Ok(success)
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::SqlitePool;
    use time::macros::{date, datetime};

    async fn setup(
        pool: &SqlitePool,
        user_max_reservations: u8,
        user_max_guest_reservations: u8,
        waiting_capacity: u8,
    ) -> (Location, User, User) {
        sqlx::query!(
            r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, null, FALSE);
        insert into users (id, email, name, password_hash, role_id, has_key)
        VALUES (1000, 'test@test.com', 'Test', '', 100, FALSE),
        (2000, 'hello@test.com', 'Test', '', 100, FALSE);

        insert into locations (name, slot_capacity, waiting_capacity, slots_start_hour, slot_duration, slots_per_day, alt_slots_start_hour, alt_slot_duration, alt_slots_per_day)
        VALUES ('test_location', 1, $3, 18, 2, 2, 10, 3, 4);
        "#, user_max_reservations, user_max_guest_reservations, waiting_capacity
        ).execute(pool).await.unwrap();

        let location = query_as!(
            Location,
            "select * from locations where name = 'test_location'"
        )
        .fetch_one(pool)
        .await
        .expect("No locations found");

        let user1 = query_as!(User, "select * from users_with_role where id = 1000")
            .fetch_one(pool)
            .await
            .unwrap();

        let user2 = query_as!(User, "select * from users_with_role where id = 2000")
            .fetch_one(pool)
            .await
            .unwrap();

        (location, user1, user2)
    }

    #[sqlx::test]
    async fn no_guest(pool: SqlitePool) {
        let (location, user_1, user_2) = setup(&pool, 2, 0, 0).await;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date_1 = date!(2024 - 07 - 11);
        let date_2 = date!(2024 - 07 - 12);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18).await,
            Err(ReservationError::AlreadyExists { cancelled: false })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date_1, 18).await,
            Err(ReservationError::SlotFull)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 20).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_2, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_2, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn with_guest(pool: SqlitePool) {
        let (location, user, _) = setup(&pool, 1, 2, 0).await;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date_1 = date!(2024 - 07 - 11);
        let date_2 = date!(2024 - 07 - 12);

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_1, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_1, 18).await,
            Err(ReservationError::AlreadyExists { cancelled: false })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_1, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_2, 18).await,
            Ok(ReservationSuccess::Guest)
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_2, 20).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn only_guest(pool: SqlitePool) {
        let (location, user_1, user_2) = setup(&pool, 0, 2, 1).await;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date_1 = date!(2024 - 07 - 11);
        let date_2 = date!(2024 - 07 - 12);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date_1, 18).await,
            Err(ReservationError::SlotFull)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18).await,
            Err(ReservationError::AlreadyExists { cancelled: false })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_2, 18).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn too_late(pool: SqlitePool) {
        let (location, user, _) = setup(&pool, 1, 0, 0).await;

        let now_good = datetime!(2024-07-11 16:59:00 +00:00:00);
        let now_too_late = datetime!(2024-07-11 17:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now_good, &user, date, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now_too_late, &user, date, 18).await,
            Err(ReservationError::Other(
                "Rezervările se fac cu cel putin o oră înainte"
            ))
        );
    }

    #[sqlx::test]
    async fn replace_guest(pool: SqlitePool) {
        let (location, user_1, user_2) = setup(&pool, 1, 1, 0).await;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        // It should replace the guest
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 20).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: true
            })
        );
    }

    #[sqlx::test]
    async fn free_days(pool: SqlitePool) {
        let (location, user, _) = setup(&pool, 1, 1, 0).await;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);
        let weekend = date!(2024 - 07 - 13);

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 20).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, weekend, 10).await,
            Err(ReservationError::NoMoreReservation)
        );
    }

    #[sqlx::test]
    async fn restrictions(pool: SqlitePool) {
        let (location, user, _) = setup(&pool, 1, 1, 1).await;

        query!("insert into restrictions (message, location, date, hour) values ('res1', $1, '2024-07-11', NULL), ('res2', $1, '2024-07-12', 18)", location.id)
            .execute(&pool)
            .await
            .unwrap();

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date_1 = date!(2024 - 07 - 11);
        let date_2 = date!(2024 - 07 - 12);

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_1, 18).await,
            Err(ReservationError::Restriction("res1".to_string()))
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_1, 20).await,
            Err(ReservationError::Restriction("res1".to_string()))
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_2, 18).await,
            Err(ReservationError::Restriction("res2".to_string()))
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_2, 20).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );
    }

    #[sqlx::test]
    async fn in_waiting(pool: SqlitePool) {
        let (location, user_1, user_2) = setup(&pool, 1, 1, 1).await;
        query!(
            "insert into users (id, email, name, password_hash, role_id, has_key)
            VALUES (3000, 'test3@test.com', 'Test3', '', 100, FALSE)"
        )
        .execute(&pool)
        .await
        .unwrap();
        let user_3 = query_as!(User, "select * from users_with_role where id = 3000")
            .fetch_one(&pool)
            .await
            .unwrap();

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18).await,
            Ok(ReservationSuccess::InWaiting)
        );

        // Will attempt as normal reservation
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18).await,
            Err(ReservationError::SlotFull)
        );

        // Use up the reservation
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 20).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // Attempt as guest
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18).await,
            Err(ReservationError::SlotFull)
        );
    }
}
