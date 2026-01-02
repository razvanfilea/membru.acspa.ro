use super::*;
use sqlx::{SqlitePool, query, query_as};
use time::macros::{date, datetime};

async fn setup(
    pool: &SqlitePool,
    user_max_reservations: u8,
    user_max_guest_reservations: u8,
) -> sqlx::Result<(Location, User, User, User)> {
    query!(
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
    .await?;

    let location = query_as!(
        Location,
        "select * from locations where name = 'test_location'"
    )
    .fetch_one(pool)
    .await?;

    let user1 = query_as!(User, "select * from users_with_role where id = 1000")
        .fetch_one(pool)
        .await?;
    let user2 = query_as!(User, "select * from users_with_role where id = 2000")
        .fetch_one(pool)
        .await?;
    let user3 = query_as!(User, "select * from users_with_role where id = 3000")
        .fetch_one(pool)
        .await?;

    Ok((location, user1, user2, user3))
}

mod validation {
    use super::*;

    #[sqlx::test]
    async fn should_reject_past_dates_and_invalid_hours(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 0).await?;

        // Context: Booking for today (11th) at 18:00
        let now = datetime!(2024-07-11 16:59:00 +00:00:00);
        let target_date = date!(2024 - 07 - 11);

        // 1. Past Date
        assert_eq!(
            create_reservation(
                &pool,
                &location,
                now,
                &user,
                date!(2024 - 07 - 10),
                18,
                None
            )
            .await,
            Err(ReservationError::Other(
                "Nu poți face o rezervare pentru o zi din trecut"
            ))
        );

        // 2. Too Late (Less than 1 hour before)
        let now_too_late = datetime!(2024-07-11 17:00:00 +00:00:00);
        assert_eq!(
            create_reservation(&pool, &location, now_too_late, &user, target_date, 18, None).await,
            Err(ReservationError::Other(
                "Rezervările se fac cu cel putin o oră înainte"
            ))
        );

        // 3. Invalid Hour (e.g., 17:00 when starts at 18:00)
        assert_eq!(
            create_reservation(&pool, &location, now, &user, target_date, 17, None).await,
            Err(ReservationError::Other(
                "Ora pentru rezervare nu este validă"
            ))
        );

        // 4. Valid Booking
        assert_eq!(
            create_reservation(&pool, &location, now, &user, target_date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        Ok(())
    }
}

mod core_booking {
    use super::*;

    #[sqlx::test]
    async fn member_booking_standard_flow(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, _) = setup(&pool, 2, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date_1 = date!(2024 - 07 - 11);
        let date_2 = date!(2024 - 07 - 12);

        // 1. Member books successfully
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 2. Duplicate booking fails (soft check)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18, None).await,
            Err(ReservationError::AlreadyExists { cancelled: false })
        );

        // 3. Second user goes to waiting list (Capacity is 1 by default in setup)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date_1, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        // 4. User 1 books another slot (Quota allows 2)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 5. User 1 quota exceeded (Try 3rd booking on next day)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_2, 18, None).await,
            Err(ReservationError::NoMoreReservations)
        );

        Ok(())
    }

    #[sqlx::test]
    async fn member_booking_with_guest_quota(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 2).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // 1. Member Reservation
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 2. Guest Reservation (Success)
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 20, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // 3. Next day guest reservation
        let date_next = date!(2024 - 07 - 12);
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_next, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // 4. Guest quota exceeded
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_next, 20, None).await,
            Err(ReservationError::NoMoreReservations)
        );

        Ok(())
    }

    #[sqlx::test]
    async fn guest_only_user_flow(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, _) = setup(&pool, 0, 2).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // 1. User 1 books as Guest (0 member slots)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // 2. User 2 (Guest) goes to Waiting (Capacity 1 full)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: true })
        );

        Ok(())
    }
}

mod queue_logic {
    use super::*;

    #[sqlx::test]
    async fn member_bumps_active_guest_to_waiting(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_member, user_guest, _) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // 1. Fill slots: Member at 18:00, Guest at 20:00
        // (Note: Capacity is 1, so these must be different slots)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_member, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user_member, date, 20, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // 2. New Member enters at 20:00 -> Should bump the Guest
        // We use user_guest (who has a Member role in setup) to bump user_member's Guest spot
        assert_eq!(
            create_reservation(&pool, &location, now, &user_guest, date, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: true
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn guest_bump_policy_should_be_lifo(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 0, 5).await?;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // Manually increase capacity to 2 to allow two active guests
        let location = query_as!(
            Location,
            "update locations set slot_capacity = 2 where id = $1 returning *",
            location.id
        )
        .fetch_one(&pool)
        .await?;

        // 1. User 1 books Guest A (Oldest)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // FORCE DISTINCT TIMESTAMPS: Backdate User 1's reservation by 1 hour.
        query!(
            "update reservations set created_at = datetime('now', '-1 hour') where user_id = $1",
            user_1.id
        )
        .execute(&pool)
        .await?;

        // 2. User 2 books Guest B (Newest)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // 3. Update Role to give Member Access
        // We update the role now so User 3 counts as a Member.
        // User 1 & 2 are already recorded as Guests in the DB, so they remain Guests.
        query!("update user_roles set reservations = 1 where id = 100")
            .execute(&pool)
            .await?;

        // 4. User 3 (Member) joins.
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: true
            })
        );

        // 5. Verify the NEWEST guest (User 2) was bumped to waiting list
        let guest2_res = query!(
            "select in_waiting from reservations where user_id = $1",
            user_2.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            guest2_res.in_waiting,
            "The newest guest (User 2) should be bumped"
        );

        // 6. Verify the OLDEST guest (User 1) is still active
        let guest1_res = query!(
            "select in_waiting from reservations where user_id = $1",
            user_1.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            !guest1_res.in_waiting,
            "The older guest (User 1) should remain active"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn waiting_list_promotion_should_be_fifo(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // 1. User 1 takes the only slot
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 2. User 2 joins waiting list
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        // Force time difference: User 2 has been waiting longer
        query!(
            "update reservations set created_at = datetime('now', '-1 hour') where user_id = $1",
            user_2.id
        )
        .execute(&pool)
        .await?;

        // 3. User 3 joins waiting list (Newer waiter)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        // 4. User 1 cancels
        let res =
            cancel_reservation(pool.begin().await?, &location, date, 18, user_1.id, None).await?;
        assert!(res);

        // 5. User 2 (Oldest Waiter) should be promoted
        let u2 = query!(
            "select in_waiting from reservations where user_id = $1",
            user_2.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            !u2.in_waiting,
            "The user who waited longer should be promoted"
        );

        // 6. User 3 (Newer Waiter) should still be waiting
        let u3 = query!(
            "select in_waiting from reservations where user_id = $1",
            user_3.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            u3.in_waiting,
            "The user who joined later should still be waiting"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn referral_should_ignore_user_limits(pool: SqlitePool) -> sqlx::Result<()> {
        // 1. Setup: User has ABSOLUTELY NO quota (0 member, 0 guest)
        // This ensures any success is due to the referral logic, not personal limits.
        let (location, user, _, _) = setup(&pool, 0, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);
        // 2. Control Check: Verify a regular booking FAILS
        // If this passes, our test setup is wrong.
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::NoMoreReservations),
            "Standard reservation must fail when quota is 0"
        );

        // 3. Action: Create a non-special Referral
        let name = "Guest Name";
        let referral = Referral {
            is_special: false,
            created_for: name,
        };
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, Some(referral)).await,
            Ok(ReservationSuccess::Guest),
            "Referral should succeed and result in a Guest slot, ignoring the user's 0 quota"
        );

        // 4. Verify Database State
        let saved_reservation = query!(
            "select created_for, as_guest from reservations where user_id = $1",
            user.id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_reservation.created_for, Some(name.to_string()),);
        assert!(saved_reservation.as_guest); // Should be a guest reservation

        Ok(())
    }
}

mod cancellation {
    use super::*;

    #[sqlx::test]
    async fn cancel_restores_quota_and_blocks_immediate_rebook(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 1. Cancel
        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None).await?);

        // 2. Verify Cancelled in DB
        let saved = query!(
            "select cancelled from reservations where user_id = $1",
            user.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(saved.cancelled);

        // 3. Try to rebook immediately -> Expect Error (AlreadyExists cancelled: true)
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::AlreadyExists { cancelled: true })
        );

        // 4. Try to book another day (Quota should be restored)
        let date_2 = date!(2024 - 07 - 12);
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_2, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn cancel_hard_deletes_named_guests(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 0).await?;
        let date = date!(2024 - 07 - 11);
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let name = "External Name";
        let referral = Referral {
            is_special: true,
            created_for: name,
        };

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, Some(referral)).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // Cancel
        let tx = pool.begin().await?;
        let res = cancel_reservation(tx, &location, date, 18, user.id, Some(name)).await?;
        assert!(res);

        // Verify Hard Delete (Count should be 0)
        let count = query!(
            "select count(*) as c from reservations where created_for = $1",
            name
        )
        .fetch_one(&pool)
        .await?
        .c;
        assert_eq!(count, 0);
        Ok(())
    }

    #[sqlx::test]
    async fn cancel_targets_specific_slot_only(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 5, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // Create two reservations (18:00 and 20:00)
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None).await?);

        // Verify it's cancelled
        let res_18 = query!("select cancelled from reservations where hour = 18")
            .fetch_one(&pool)
            .await?;
        assert!(res_18.cancelled);

        // Verify 20:00 is ACTIVE
        let res_20 = query!("select cancelled from reservations where hour = 20")
            .fetch_one(&pool)
            .await?;
        assert!(!res_20.cancelled);

        Ok(())
    }

    #[sqlx::test]
    async fn cancel_promotes_waiting_member_over_waiting_guest(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // 1. User 1 takes the slot
        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 2. User 2 (Member) uses their main slot elsewhere (20:00)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // 2b. User 2 joins waiting list at 18:00 (Must be Guest now)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: true })
        );

        // 3. User 3 (Member) joins waiting list at 18:00
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        // 4. User 1 cancels
        let res =
            cancel_reservation(pool.begin().await?, &location, date, 18, user_1.id, None).await?;
        assert!(res);

        // 5. Verify User 3 (Member) got promoted, NOT User 2 (Guest)
        let user_3_status = query!(
            "select in_waiting from reservations where user_id = $1 and hour = 18",
            user_3.id
        )
        .fetch_one(&pool)
        .await?
        .in_waiting;
        assert!(!user_3_status, "Member should be promoted");

        let user_2_status = query!(
            "select in_waiting from reservations where user_id = $1 and hour = 18",
            user_2.id
        )
        .fetch_one(&pool)
        .await?
        .in_waiting;
        assert!(user_2_status, "Guest should still be waiting");

        Ok(())
    }
}

mod constraints_and_schedule {
    use super::*;

    #[sqlx::test]
    async fn should_apply_database_restrictions(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);

        query!("insert into restrictions (message, location, date, hour) values ('res1', $1, '2024-07-11', NULL), ('res2', $1, '2024-07-12', 18)", location.id)
            .execute(&pool).await?;

        // Day wide restriction
        assert_eq!(
            create_reservation(
                &pool,
                &location,
                now,
                &user,
                date!(2024 - 07 - 11),
                18,
                None
            )
            .await,
            Err(ReservationError::Restriction("res1".to_string()))
        );

        // Hour specific restriction
        assert_eq!(
            create_reservation(
                &pool,
                &location,
                now,
                &user,
                date!(2024 - 07 - 12),
                18,
                None
            )
            .await,
            Err(ReservationError::Restriction("res2".to_string()))
        );

        // Unrestricted hour
        assert_eq!(
            create_reservation(
                &pool,
                &location,
                now,
                &user,
                date!(2024 - 07 - 12),
                20,
                None
            )
            .await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn holidays_override_standard_schedule(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-10 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        // Create Holiday with custom capacity of 2 and different start hour (10:00)
        query!("insert into schedule_overrides (date, type, slots_start_hour, slot_duration, slots_per_day, slot_capacity) values ('2024-07-11', 'holiday', 10, 3, 4, 2)")
            .execute(&pool).await?;

        // 1. Standard hour (18:00) should now be invalid
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::Other(
                "Ora pentru rezervare nu este validă"
            ))
        );

        // 2. New holiday hour (10:00) should be valid
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 10, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        Ok(())
    }

    #[sqlx::test]
    async fn weekend_creates_standard_reservations(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11); // Thursday
        let weekend = date!(2024 - 07 - 13); // Saturday

        // Normal day
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 20, None).await,
            Ok(ReservationSuccess::Guest)
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user, weekend, 10, None).await,
            Err(ReservationError::NoMoreReservations)
        );

        Ok(())
    }
}
