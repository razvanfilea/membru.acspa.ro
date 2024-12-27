use super::*;
use sqlx::{query_as, SqlitePool};
use time::macros::{date, datetime};

async fn setup(
    pool: &SqlitePool,
    user_max_reservations: u8,
    user_max_guest_reservations: u8,
) -> (Location, User, User) {
    sqlx::query!(
        r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, null, FALSE);
        insert into users (id, email, name, password_hash, role_id, has_key)
        VALUES (1000, 'test@test.com', 'Test', '', 100, FALSE),
        (2000, 'hello@test.com', 'Test', '', 100, FALSE);

        insert into locations (name, slot_capacity, slots_start_hour, slot_duration, slots_per_day)
        VALUES ('test_location', 1, 18, 2, 2);
        "#,
        user_max_reservations,
        user_max_guest_reservations
    )
    .execute(pool)
    .await
    .unwrap();

    let location = query_as!(
        Location,
        "select * from locations where name = 'test_location'"
    )
    .fetch_one(pool)
    .await
    .expect("No locations found");

    let user1 = query_as("select * from users_with_role where id = 1000")
        .fetch_one(pool)
        .await
        .unwrap();

    let user2 = query_as("select * from users_with_role where id = 2000")
        .fetch_one(pool)
        .await
        .unwrap();

    (location, user1, user2)
}

#[sqlx::test]
async fn no_guest(pool: SqlitePool) {
    let (location, user_1, user_2) = setup(&pool, 2, 0).await;

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
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date_1, 20).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date_2, 18).await,
        Err(ReservationError::NoMoreReservations)
    );
    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date_2, 20).await,
        Err(ReservationError::NoMoreReservations)
    );
}

#[sqlx::test]
async fn with_guest(pool: SqlitePool) {
    let (location, user, _) = setup(&pool, 1, 2).await;

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
        Err(ReservationError::NoMoreReservations)
    );
}

#[sqlx::test]
async fn only_guest(pool: SqlitePool) {
    let (location, user_1, user_2) = setup(&pool, 0, 2).await;

    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date_1 = date!(2024 - 07 - 11);
    let date_2 = date!(2024 - 07 - 12);

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date_1, 18).await,
        Ok(ReservationSuccess::Guest)
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date_1, 18).await,
        Ok(ReservationSuccess::InWaiting { as_guest: true })
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
        Err(ReservationError::NoMoreReservations)
    );
}

#[sqlx::test]
async fn too_late(pool: SqlitePool) {
    let (location, user, _) = setup(&pool, 1, 0).await;

    let now_too_late = datetime!(2024-07-11 17:00:00 +00:00:00);
    let way_too_late = datetime!(2024-07-11 19:00:00 +00:00:00);
    let now_good = datetime!(2024-07-11 16:59:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    assert_eq!(
        create_reservation(&pool, &location, now_too_late, &user, date, 18).await,
        Err(ReservationError::Other(
            "Rezervările se fac cu cel putin o oră înainte"
        ))
    );

    assert_eq!(
        create_reservation(&pool, &location, way_too_late, &user, date, 18).await,
        Err(ReservationError::Other(
            "Rezervările se fac cu cel putin o oră înainte"
        ))
    );

    assert_eq!(
        create_reservation(&pool, &location, now_good, &user, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
}

#[sqlx::test]
async fn replace_guest(pool: SqlitePool) {
    let (location, user_1, user_2) = setup(&pool, 1, 1).await;

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
async fn weekend(pool: SqlitePool) {
    let (location, user, _) = setup(&pool, 1, 1).await;

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
        Err(ReservationError::NoMoreReservations)
    );
}

#[sqlx::test]
async fn restrictions(pool: SqlitePool) {
    let (location, user, _) = setup(&pool, 1, 1).await;

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
    let (location, user_1, user_2) = setup(&pool, 1, 1).await;
    query(
        "insert into users (id, email, name, password_hash, role_id, has_key)
            VALUES (3000, 'test3@test.com', 'Test3', '', 100, FALSE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let user_3 = query_as("select * from users_with_role where id = 3000")
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
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_3, date, 18).await,
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_3, date, 20).await,
        Ok(ReservationSuccess::Guest)
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_3, date, 18).await,
        Err(ReservationError::AlreadyExists { cancelled: false })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 20).await,
        Ok(ReservationSuccess::InWaiting { as_guest: true })
    );
}

#[sqlx::test]
async fn alternative_day(pool: SqlitePool) {
    let (location, user_1, user_2) = setup(&pool, 1, 1).await;

    query("insert into alternative_days (date, type, slots_start_hour, slot_duration, slots_per_day) values ('2024-07-11', 'holiday', 10, 3, 4)")
        .execute(&pool)
        .await
        .unwrap();

    let now = datetime!(2024-07-10 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 18).await,
        Err(ReservationError::Other(
            "Ora pentru rezervare nu este validă"
        ))
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 10).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 10).await,
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 13).await,
        Ok(ReservationSuccess::Guest)
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 19).await,
        Ok(ReservationSuccess::Guest)
    );
}

#[sqlx::test]
async fn alternative_day_custom_capacity(pool: SqlitePool) {
    let (location, user_1, user_2) = setup(&pool, 1, 1).await;

    query("insert into alternative_days (date, type, slots_start_hour, slot_duration, slots_per_day, slot_capacity) values ('2024-07-11', 'holiday', 10, 3, 4, 2)")
        .execute(&pool)
        .await
        .unwrap();
    let now = datetime!(2024-07-10 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 10).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 10).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
}
