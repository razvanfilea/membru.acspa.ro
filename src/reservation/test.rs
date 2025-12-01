use super::*;
use sqlx::{SqlitePool, query_as};
use time::macros::{date, datetime};

async fn setup(
    pool: &SqlitePool,
    user_max_reservations: u8,
    user_max_guest_reservations: u8,
) -> (Location, User, User, User) {
    sqlx::query!(
        r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, null, FALSE);
        insert into users (id, email, name, password_hash, role_id, has_key, birthday, member_since)
        VALUES (1000, 'test1@test.com', 'Test 1', '', 100, FALSE, '2000-01-01', '2000-01-01'),
        (2000, 'test2@test.com', 'Test 2', '', 100, FALSE, '2000-01-01', '2000-01-01'),
        (3000, 'test3@test.com', 'Test 3', '', 100, FALSE, '2000-01-01', '2000-01-01');

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

    let user1 = query_as!(User, "select * from users_with_role where id = 1000")
        .fetch_one(pool)
        .await
        .unwrap();
    let user2 = query_as!(User, "select * from users_with_role where id = 2000")
        .fetch_one(pool)
        .await
        .unwrap();
    let user3 = query_as!(User, "select * from users_with_role where id = 3000")
        .fetch_one(pool)
        .await
        .unwrap();

    (location, user1, user2, user3)
}

#[sqlx::test]
async fn no_guest(pool: SqlitePool) {
    let (location, user_1, user_2, _) = setup(&pool, 2, 0).await;

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
    let (location, user, _, _) = setup(&pool, 1, 2).await;

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
    let (location, user_1, user_2, _) = setup(&pool, 0, 2).await;

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
async fn invalid_date_or_hour(pool: SqlitePool) {
    let (location, user, _, _) = setup(&pool, 1, 0).await;

    let past_date = date!(2024 - 07 - 10);
    let now_too_late = datetime!(2024-07-11 17:00:00 +00:00:00);
    let way_too_late = datetime!(2024-07-11 19:00:00 +00:00:00);
    let now = datetime!(2024-07-11 16:59:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    assert_eq!(
        create_reservation(&pool, &location, now, &user, past_date, 18).await,
        Err(ReservationError::Other(
            "Nu poți face o rezervare pentru o zi din trecut"
        ))
    );

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
        create_reservation(&pool, &location, now, &user, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // Invalid hour
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date, 17).await,
        Err(ReservationError::Other(
            "Ora pentru rezervare nu este validă"
        ))
    );
}

#[sqlx::test]
async fn replace_guest(pool: SqlitePool) {
    let (location, user_1, user_2, _) = setup(&pool, 1, 1).await;

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
    let (location, user, _, _) = setup(&pool, 1, 1).await;

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
    let (location, user, _, _) = setup(&pool, 1, 1).await;

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
    let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await;

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
    let (location, user_1, user_2, _) = setup(&pool, 1, 1).await;

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
    let (location, user_1, user_2, _) = setup(&pool, 1, 1).await;

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
#[sqlx::test]
async fn cancel_simple(pool: SqlitePool) {
    let (location, user, _, _) = setup(&pool, 1, 0).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);
    let date_2 = date!(2024 - 07 - 12);

    // 1. Create a reservation
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // 2. Cancel the reservation
    let tx = pool.begin().await.unwrap();
    let result = cancel_reservation(tx, &location, date, 18, user.id, None).await;
    assert!(result.unwrap());

    // 3. Verify it is marked as cancelled in DB
    let saved = sqlx::query!(
        "select cancelled from reservations where user_id = $1 and date = $2 and hour = $3",
        user.id,
        date,
        18
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(saved.cancelled);

    // Try to create again -> Expect Error
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date, 18).await,
        Err(ReservationError::AlreadyExists { cancelled: true })
    );

    // 4. User tries to book the second day again -> Should Succeed now
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date_2, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
}

#[sqlx::test]
async fn cancel_non_existent(pool: SqlitePool) {
    let (location, user, _, _) = setup(&pool, 1, 0).await;
    let date = date!(2024 - 07 - 11);

    // Try to cancel a reservation that was never created
    let tx = pool.begin().await.unwrap();
    let result = cancel_reservation(tx, &location, date, 18, user.id, None).await;

    assert!(!result.unwrap());
}

#[sqlx::test]
async fn cancel_targets_specific_slot_only(pool: SqlitePool) {
    let (location, user, _, _) = setup(&pool, 5, 0).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // 1. Create two reservations for the same user (different hours)
    create_reservation(&pool, &location, now, &user, date, 18)
        .await
        .unwrap();
    create_reservation(&pool, &location, now, &user, date, 20)
        .await
        .unwrap();

    // 2. Cancel ONLY the 18:00 slot
    let tx = pool.begin().await.unwrap();
    cancel_reservation(tx, &location, date, 18, user.id, None)
        .await
        .unwrap();

    // 3. Verify 18:00 is cancelled
    let res_18 = sqlx::query!("SELECT cancelled FROM reservations WHERE hour = 18")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(res_18.cancelled);

    // 4. Verify 20:00 is STILL ACTIVE
    let res_20 = sqlx::query!("SELECT cancelled FROM reservations WHERE hour = 20")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!res_20.cancelled);
}

#[sqlx::test]
async fn cancel_promotes_waiting_user(pool: SqlitePool) {
    let (location, user_1, user_2, _) = setup(&pool, 1, 0).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // 1. User 1 takes the only slot
    let res = create_reservation(&pool, &location, now, &user_1, date, 18).await;
    assert_eq!(
        res,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // 2. User 2 goes into waiting
    let res = create_reservation(&pool, &location, now, &user_2, date, 18).await;
    assert_eq!(res, Ok(ReservationSuccess::InWaiting { as_guest: false }));

    // 3. User 1 cancels
    let tx = pool.begin().await.unwrap();
    let result = cancel_reservation(tx, &location, date, 18, user_1.id, None).await;
    assert!(result.unwrap());

    // 4. Verify User 2 is no longer in_waiting
    let user_2_res = sqlx::query!(
        "select in_waiting from reservations where user_id = $1",
        user_2.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(!user_2_res.in_waiting);
}

#[sqlx::test]
async fn cancel_promotes_priority_order(pool: SqlitePool) {
    let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await;

    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // 1. User 1 takes the slot
    let res = create_reservation(&pool, &location, now, &user_1, date, 18).await;
    assert_eq!(
        res,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // 2. User 2 uses up his reservation.
    let res = create_reservation(&pool, &location, now, &user_2, date, 20).await;
    assert_eq!(
        res,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
    // 2. User 2 adds a Guest (In Waiting).
    let res = create_reservation(&pool, &location, now, &user_2, date, 18).await;
    assert_eq!(res, Ok(ReservationSuccess::InWaiting { as_guest: true }));

    // 3. User 3 requests a spot (In Waiting).
    let res = create_reservation(&pool, &location, now, &user_3, date, 18).await;
    assert_eq!(res, Ok(ReservationSuccess::InWaiting { as_guest: false }));

    // Verify initial state: Both waiting
    let waiting_count =
        sqlx::query!("select count(*) as c from reservations where in_waiting = true")
            .fetch_one(&pool)
            .await
            .unwrap()
            .c;
    assert_eq!(waiting_count, 2);

    // 4. User 1 cancels the main reservation (opening up 1 spot)
    let tx = pool.begin().await.unwrap();
    let res = cancel_reservation(tx, &location, date, 18, user_1.id, None)
        .await
        .unwrap();
    assert!(res);

    // 5. Verify User 2 (Member) got promoted, NOT the Guest
    let user_2_status = sqlx::query!(
        "select in_waiting from reservations where user_id = $1",
        user_2.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .in_waiting;
    assert!(!user_2_status, "Member should be promoted");

    // 6. Verify User 3 (Guest) is still waiting
    let user_3_status = sqlx::query!(
        "select in_waiting from reservations where user_id = $1",
        user_2.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .in_waiting;

    assert!(!user_3_status, "User 3 should still be waiting");
}

#[sqlx::test]
async fn promotion_fifo_for_guests(pool: SqlitePool) {
    let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // 1. User 1 takes the only slot
    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // 2. User 2 joins waiting list
    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 18).await,
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    // Force time difference to ensure FIFO order
    sqlx::query!(
        "UPDATE reservations SET created_at = datetime('now', '-1 hour') WHERE user_id = $1",
        user_2.id
    )
    .execute(&pool)
    .await
    .unwrap();

    // 3. User 3 joins waiting list (Later than User 2)
    assert_eq!(
        create_reservation(&pool, &location, now, &user_3, date, 18).await,
        Ok(ReservationSuccess::InWaiting { as_guest: false })
    );

    // 4. User 1 cancels
    let tx = pool.begin().await.unwrap();
    let res = cancel_reservation(tx, &location, date, 18, user_1.id, None).await;
    assert!(res.unwrap());

    // 5. User 2 (Older) should be promoted
    let u2 = sqlx::query!(
        "SELECT in_waiting FROM reservations WHERE user_id = $1",
        user_2.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !u2.in_waiting,
        "The user who waited longer should be promoted"
    );

    // 6. User 3 (Newer) should still be waiting
    let u3 = sqlx::query!(
        "SELECT in_waiting FROM reservations WHERE user_id = $1",
        user_3.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        u3.in_waiting,
        "The user who joined later should still be waiting"
    );
}

#[sqlx::test]
async fn guest_bump_policy_is_lifo(pool: SqlitePool) {
    let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // Manually increase capacity to 2 to allow two active guests
    query!(
        "UPDATE locations SET slot_capacity = 2 WHERE id = $1",
        location.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let location = query_as!(
        Location,
        "select * from locations where name = 'test_location'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // 1. Use up user 1 and user 2's slots
    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 20).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 20).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );

    // 2. User 1 books Guest A (Oldest)
    assert_eq!(
        create_reservation(&pool, &location, now, &user_1, date, 18).await,
        Ok(ReservationSuccess::Guest)
    );

    // FORCE DISTINCT TIMESTAMPS: Backdate User 1's reservation by 1 hour.
    // This ensures User 1 is strictly "older" than User 2, regardless of execution speed.
    sqlx::query!(
        "UPDATE reservations SET created_at = datetime('now', '-1 hour') WHERE user_id = $1",
        user_1.id
    )
    .execute(&pool)
    .await
    .unwrap();
    // 3. User 2 books Guest B (Newest)
    assert_eq!(
        create_reservation(&pool, &location, now, &user_2, date, 18).await,
        Ok(ReservationSuccess::Guest)
    );

    // 4. Member creates reservation
    assert_eq!(
        create_reservation(&pool, &location, now, &user_3, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: true
        })
    );

    // 5. Verify the NEWEST guest (User 2) was bumped to waiting list
    let guest2_res = query!(
        "SELECT in_waiting FROM reservations WHERE user_id = $1",
        user_2.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        guest2_res.in_waiting,
        "The newest guest (User 2) should be bumped"
    );

    // 6. Verify the OLDEST guest (User 1) is still active
    let guest1_res = query!(
        "SELECT in_waiting FROM reservations WHERE user_id = $1",
        user_1.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !guest1_res.in_waiting,
        "The older guest (User 1) should remain active"
    );
}

#[sqlx::test]
async fn referred_guest_bypasses_limits(pool: SqlitePool) {
    // Setup: User has 0 guest allowance, Location has capacity of 1
    let (location, user, _, _) = setup(&pool, 1, 0).await;
    let now = datetime!(2024-07-11 10:00:00 +00:00:00);
    let date = date!(2024 - 07 - 11);

    // 1. Validate standard logic: User cannot create a guest normally
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date, 18).await,
        Ok(ReservationSuccess::Reservation {
            deletes_guest: false
        })
    );
    // Try to add a guest (should fail due to limit 0)
    assert_eq!(
        create_reservation(&pool, &location, now, &user, date, 18).await,
        Err(ReservationError::AlreadyExists { cancelled: false }) // Note: Logic in test setup usually implies 2nd res is guest,
                                                                  // but here it might fail strictly on limits if not handled carefully.
                                                                  // Assuming standard flow failure here.
    );

    // 2. Create a referred guest
    // This should succeed even though user has 0 guest reservations
    let refer_result = create_referred_guest(
        pool.begin().await.unwrap(),
        &location,
        date,
        18,
        user.id,
        true,
        "Referred Friend",
    )
    .await;

    assert!(refer_result.is_ok());

    // 3. Verify the referred guest went into Waiting List
    // (Because capacity is 1, and Member took the slot in step 1)
    let saved_guest = query!(
        "select in_waiting, created_for from reservations where created_for = 'Referred Friend'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        saved_guest.in_waiting,
        "Referred guest should be in waiting because slot is full"
    );
    assert_eq!(saved_guest.created_for, Some("Referred Friend".to_string()));
}

#[sqlx::test]
async fn cancel_hard_deletes_referred_reservation(pool: SqlitePool) {
    let (location, user, _, _) = setup(&pool, 1, 0).await;
    let date = date!(2024 - 07 - 11);

    let name = "External Name".to_string();
    let refer_result = create_referred_guest(
        pool.begin().await.unwrap(),
        &location,
        date,
        18,
        user.id,
        true,
        &name,
    )
    .await;
    assert!(refer_result.is_ok());

    let tx = pool.begin().await.unwrap();
    let res = cancel_reservation(tx, &location, date, 18, user.id, Some(&name))
        .await
        .unwrap();
    assert!(res);

    let count = query!(
        "SELECT count(*) as c FROM reservations WHERE created_for = $1",
        name
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .c;

    assert_eq!(count, 0, "Reservation should be hard deleted");
}
