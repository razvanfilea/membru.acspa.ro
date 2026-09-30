use super::*;
use sqlx::{SqlitePool, query, query_as, query_scalar};
use time::macros::{date, datetime};

async fn setup(
    pool: &SqlitePool,
    user_max_reservations: u8,
    user_max_guest_reservations: u8,
) -> sqlx::Result<(Location, User, User, User)> {
    query!(
        r#"
        insert into user_roles VALUES (100, 'Test Role', $1, $2, null, FALSE, null);
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

    #[test]
    fn cancellable_until_the_slot_starts() {
        let now = datetime!(2024-07-11 17:59:00 +00:00:00);
        assert!(is_cancellable_at(date!(2024 - 07 - 11), 18, None, now));
        assert!(is_cancellable_at(date!(2024 - 07 - 12), 8, None, now));
        assert!(!is_cancellable_at(date!(2024 - 07 - 11), 17, None, now));

        let now = datetime!(2024-07-11 18:00:00 +00:00:00);
        assert!(!is_cancellable_at(date!(2024 - 07 - 11), 18, None, now));
        assert!(!is_cancellable_at(date!(2024 - 07 - 10), 20, None, now));
    }

    #[test]
    fn cancellable_until_a_slot_with_minutes_starts() {
        let date = date!(2024 - 07 - 11);
        let minute = Some(30);

        assert!(is_cancellable_at(
            date,
            18,
            minute,
            datetime!(2024-07-11 18:00:00 +00:00:00)
        ));
        assert!(is_cancellable_at(
            date,
            18,
            minute,
            datetime!(2024-07-11 18:29:00 +00:00:00)
        ));
        assert!(!is_cancellable_at(
            date,
            18,
            minute,
            datetime!(2024-07-11 18:30:00 +00:00:00)
        ));
        assert!(!is_cancellable_at(
            date,
            18,
            minute,
            datetime!(2024-07-11 19:00:00 +00:00:00)
        ));
    }

    #[sqlx::test]
    async fn should_reject_past_dates_and_invalid_hours(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 0).await?;

        let now = datetime!(2024-07-11 16:59:00 +00:00:00);
        let target_date = date!(2024 - 07 - 11);

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
            Err(ReservationError::PastDate)
        );

        // Less than 1 hour before
        let now_too_late = datetime!(2024-07-11 17:00:00 +00:00:00);
        assert_eq!(
            create_reservation(&pool, &location, now_too_late, &user, target_date, 18, None).await,
            Err(ReservationError::BookingWindowClosed)
        );

        // Slots start at 18:00
        assert_eq!(
            create_reservation(&pool, &location, now, &user, target_date, 17, None).await,
            Err(ReservationError::InvalidHour)
        );

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

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 18, None).await,
            Err(ReservationError::AlreadyExists { cancelled: false })
        );

        // Capacity is 1 by default in setup
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date_1, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date_1, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

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

        let date_next = date!(2024 - 07 - 12);
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date_next, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

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

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

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

        // Capacity is 1, so these must be different slots
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

        // user_guest has a Member role in setup, so it bumps user_member's guest spot
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

        // Capacity 2 allows two active guests
        let location = query_as!(
            Location,
            "update locations set slot_capacity = 2 where id = $1 returning *",
            location.id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        query!(
            "update reservations set created_at = datetime('now', '-1 hour') where user_id = $1",
            user_1.id
        )
        .execute(&pool)
        .await?;

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );

        // User 3 now counts as a Member, users 1 and 2 stay recorded as Guests
        query!("update user_roles set reservations = 1 where id = 100")
            .execute(&pool)
            .await?;

        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: true
            })
        );

        let guest2_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1",
            user_2.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            guest2_in_waiting,
            "The newest guest (User 2) should be bumped"
        );

        let guest1_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1",
            user_1.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            !guest1_in_waiting,
            "The older guest (User 1) should remain active"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn waiting_list_promotion_should_be_fifo(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

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

        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        let res = cancel_reservation(
            pool.begin().await?,
            &location,
            date,
            18,
            user_1.id,
            None,
            user_1.id,
        )
        .await?;
        assert!(res);

        let u2_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1",
            user_2.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            !u2_in_waiting,
            "The user who waited longer should be promoted"
        );

        let u3_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1",
            user_3.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(
            u3_in_waiting,
            "The user who joined later should still be waiting"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn referral_should_ignore_user_limits(pool: SqlitePool) -> sqlx::Result<()> {
        // No quota at all, so any success is due to the referral
        let (location, user, _, _) = setup(&pool, 0, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);
        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::NoMoreReservations),
            "Standard reservation must fail when quota is 0"
        );

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

        let saved_reservation = query!(
            "select created_for, as_guest from reservations where user_id = $1",
            user.id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_reservation.created_for, Some(name.to_string()),);
        assert!(saved_reservation.as_guest);

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

        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None, user.id).await?);

        let cancelled = query_scalar!(
            "select cancelled from reservations where user_id = $1",
            user.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(cancelled);

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::AlreadyExists { cancelled: true })
        );

        // Quota is restored after cancelling
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

        let tx = pool.begin().await?;
        let res = cancel_reservation(tx, &location, date, 18, user.id, Some(name), user.id).await?;
        assert!(res);

        let count = query_scalar!(
            "select count(*) from reservations where created_for = $1",
            name
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[sqlx::test]
    async fn cancel_targets_specific_slot_only(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 5, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

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
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None, user.id).await?);

        let res_18_cancelled = query_scalar!("select cancelled from reservations where hour = 18")
            .fetch_one(&pool)
            .await?;
        assert!(res_18_cancelled);

        let res_20_cancelled = query_scalar!("select cancelled from reservations where hour = 20")
            .fetch_one(&pool)
            .await?;
        assert!(!res_20_cancelled);

        Ok(())
    }

    #[sqlx::test]
    async fn cancel_promotes_waiting_member_over_waiting_guest(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        // User 2 uses their member slot elsewhere (20:00)
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 20, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: true })
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        let res = cancel_reservation(
            pool.begin().await?,
            &location,
            date,
            18,
            user_1.id,
            None,
            user_1.id,
        )
        .await?;
        assert!(res);

        let user_3_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1 and hour = 18",
            user_3.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(!user_3_in_waiting, "Member should be promoted");

        let user_2_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1 and hour = 18",
            user_2.id
        )
        .fetch_one(&pool)
        .await?;
        assert!(user_2_in_waiting, "Guest should still be waiting");

        Ok(())
    }

    #[sqlx::test]
    async fn remove_upcoming_uses_the_slot_start_minute(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, admin) = setup(&pool, 3, 0).await?;
        let date = date!(2024 - 07 - 11);

        query!(
            "insert into alternative_days (date, type, slots_start_hour, slots_start_minute, slot_duration, slots_per_day)
             values ('2024-07-11', 'turneu', 18, 30, 3, 1)"
        )
        .execute(&pool)
        .await?;

        create_reservation(
            &pool,
            &location,
            datetime!(2024-07-11 10:00:00 +00:00:00),
            &user,
            date,
            18,
            None,
        )
        .await
        .unwrap();

        // The slot has started at 18:30, so the reservation is kept
        let mut tx = pool.begin().await?;
        let started = datetime!(2024-07-11 18:30:00 +00:00:00);
        assert_eq!(
            remove_upcoming_reservations(&mut tx, &location, started, user.id, admin.id).await?,
            0
        );

        // At 18:10 the hour has begun but the slot hasn't
        let upcoming = datetime!(2024-07-11 18:10:00 +00:00:00);
        assert_eq!(
            remove_upcoming_reservations(&mut tx, &location, upcoming, user.id, admin.id).await?,
            1
        );
        tx.commit().await?;

        let remaining = query_scalar!("select count(*) from reservations")
            .fetch_one(&pool)
            .await?;
        assert_eq!(remaining, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn remove_upcoming_keeps_past_and_in_progress(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, admin) = setup(&pool, 3, 0).await?;
        let booked_at = datetime!(2024-07-10 10:00:00 +00:00:00);

        for (date, hour) in [
            (date!(2024 - 07 - 10), 18),
            (date!(2024 - 07 - 11), 18),
            (date!(2024 - 07 - 11), 20),
        ] {
            create_reservation(&pool, &location, booked_at, &user_1, date, hour, None)
                .await
                .unwrap();
        }
        assert_eq!(
            create_reservation(
                &pool,
                &location,
                booked_at,
                &user_2,
                date!(2024 - 07 - 11),
                20,
                None
            )
            .await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        // The 18:00 slot of the 11th is in progress
        let now = datetime!(2024-07-11 18:30:00 +00:00:00);
        let mut tx = pool.begin().await?;
        let cancelled =
            remove_upcoming_reservations(&mut tx, &location, now, user_1.id, admin.id).await?;
        tx.commit().await?;
        assert_eq!(cancelled, 1);

        let remaining = query!(
            r#"select user_id, date as "date: Date", hour, cancelled, in_waiting from reservations
            order by date, hour, user_id"#
        )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|r| (r.user_id, r.date, r.hour, r.cancelled, r.in_waiting))
        .collect::<Vec<_>>();

        assert_eq!(
            remaining,
            vec![
                (user_1.id, date!(2024 - 07 - 10), 18, false, false),
                (user_1.id, date!(2024 - 07 - 11), 18, false, false),
                (user_2.id, date!(2024 - 07 - 11), 20, false, false),
            ]
        );

        // The removed reservation keeps only its cancellation, not an extra delete
        let history = query!(
            "select event, user_id, changed_by from reservation_history where date = '2024-07-11' and hour = 20 order by id"
        )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|r| (r.event, r.user_id, r.changed_by))
        .collect::<Vec<_>>();

        assert_eq!(
            history,
            vec![
                ("created".to_string(), user_1.id, Some(user_1.id)),
                ("created".to_string(), user_2.id, Some(user_2.id)),
                ("cancelled".to_string(), user_1.id, Some(admin.id)),
                ("promoted".to_string(), user_2.id, None),
            ]
        );

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
        query!("insert into alternative_days (date, type, slots_start_hour, slot_duration, slots_per_day, slot_capacity) values ('2024-07-11', 'holiday', 10, 3, 4, 2)")
            .execute(&pool).await?;

        assert_eq!(
            create_reservation(&pool, &location, now, &user, date, 18, None).await,
            Err(ReservationError::InvalidHour)
        );

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

    #[sqlx::test]
    async fn cancel_should_respect_tournament_capacity(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await?;

        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);
        let hour = 18;

        // Tournament override for this date with capacity 2
        query!(
            "insert into alternative_days (
                type, date, description, slots_start_hour,
                slot_duration, slot_capacity, slots_per_day, consumes_reservation
            ) values (
                'turneu', $1, 'Tournament', $2,
                2, 2, 1, true
            )",
            date,
            hour
        )
        .execute(&pool)
        .await?;

        assert_eq!(
            create_reservation(&pool, &location, now, &user_1, date, hour, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            }),
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, hour, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: false
            }),
        );

        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, hour, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false }),
        );

        // Promotion must use the tournament capacity (2), not the location default (1)
        let tx = pool.begin().await?;
        let result =
            cancel_reservation(tx, &location, date, hour, user_1.id, None, user_1.id).await?;
        assert!(result, "Cancellation should succeed");

        let user_3_in_waiting = query_scalar!(
            "select in_waiting from reservations where user_id = $1",
            user_3.id
        )
        .fetch_one(&pool)
        .await?;

        assert!(
            !user_3_in_waiting,
            "FAILURE: User 3 should have been promoted to active, but is still in waiting.
             This indicates the cancellation logic used the default location capacity (1)
             instead of the tournament capacity (2)."
        );

        Ok(())
    }
}

mod history {
    use super::*;
    use crate::model::user_reservation::UserReservation;

    #[derive(Debug, PartialEq)]
    struct HistoryRow {
        event: String,
        user_id: i64,
        hour: i64,
        in_waiting: bool,
        waiting_position: Option<i64>,
        changed_by: Option<i64>,
    }

    fn row(
        event: &str,
        user_id: i64,
        in_waiting: bool,
        waiting_position: Option<i64>,
        changed_by: Option<i64>,
    ) -> HistoryRow {
        HistoryRow {
            event: event.to_string(),
            user_id,
            hour: 18,
            in_waiting,
            waiting_position,
            changed_by,
        }
    }

    async fn fetch_history(pool: &SqlitePool) -> sqlx::Result<Vec<HistoryRow>> {
        query_as!(
            HistoryRow,
            "select event, user_id, hour, in_waiting, waiting_position, changed_by
            from reservation_history order by id"
        )
        .fetch_all(pool)
        .await
    }

    #[sqlx::test]
    async fn records_waiting_positions_cancellation_and_promotion(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        for user in [&user_1, &user_2, &user_3] {
            create_reservation(&pool, &location, now, user, date, 18, None)
                .await
                .unwrap();
        }

        assert_eq!(
            fetch_history(&pool).await?,
            vec![
                row("created", user_1.id, false, None, Some(user_1.id)),
                row("created", user_2.id, true, Some(1), Some(user_2.id)),
                row("created", user_3.id, true, Some(2), Some(user_3.id)),
            ]
        );

        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user_1.id, None, user_1.id).await?);

        let history = fetch_history(&pool).await?;
        assert_eq!(
            history[3..],
            [
                row("cancelled", user_1.id, false, None, Some(user_1.id)),
                row("promoted", user_2.id, false, None, None),
            ]
        );

        Ok(())
    }

    #[sqlx::test]
    async fn records_admin_cancelling_member_booking(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, admin, _) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        create_reservation(&pool, &location, now, &user, date, 18, None)
            .await
            .unwrap();

        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None, admin.id).await?);

        assert_eq!(
            fetch_history(&pool).await?.last(),
            Some(&row("cancelled", user.id, false, None, Some(admin.id)))
        );

        Ok(())
    }

    #[sqlx::test]
    async fn records_single_delete_of_named_guest(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, admin, other_admin, _) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);
        let name = "External Name";
        let referral = Referral {
            is_special: true,
            created_for: name,
        };

        create_reservation(&pool, &location, now, &admin, date, 18, Some(referral))
            .await
            .unwrap();

        let tx = pool.begin().await?;
        assert!(
            cancel_reservation(
                tx,
                &location,
                date,
                18,
                admin.id,
                Some(name),
                other_admin.id
            )
            .await?
        );

        assert_eq!(
            fetch_history(&pool).await?,
            vec![
                row("created", admin.id, false, None, Some(admin.id)),
                row("deleted", admin.id, false, None, Some(other_admin.id)),
            ]
        );

        Ok(())
    }

    #[sqlx::test]
    async fn records_guest_moved_to_waiting(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_member, user_guest, _) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        create_reservation(&pool, &location, now, &user_member, date, 20, None)
            .await
            .unwrap();
        assert_eq!(
            create_reservation(&pool, &location, now, &user_member, date, 18, None).await,
            Ok(ReservationSuccess::Guest)
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user_guest, date, 18, None).await,
            Ok(ReservationSuccess::Reservation {
                deletes_guest: true
            })
        );

        let history = fetch_history(&pool).await?;
        assert_eq!(
            history[2],
            row("moved_to_waiting", user_member.id, true, Some(1), None)
        );
        assert_eq!(
            history[3],
            row("created", user_guest.id, false, None, Some(user_guest.id))
        );

        Ok(())
    }
    #[sqlx::test]
    async fn cancelling_twice_is_rejected_and_not_recorded(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, admin, _) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        create_reservation(&pool, &location, now, &user, date, 18, None)
            .await
            .unwrap();

        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user.id, None, user.id).await?);
        let tx = pool.begin().await?;
        assert!(!cancel_reservation(tx, &location, date, 18, user.id, None, user.id).await?);
        let tx = pool.begin().await?;
        assert!(!cancel_reservation(tx, &location, date, 18, user.id, None, admin.id).await?);

        assert_eq!(
            fetch_history(&pool).await?,
            vec![
                row("created", user.id, false, None, Some(user.id)),
                row("cancelled", user.id, false, None, Some(user.id)),
            ]
        );

        let cancelled_by = query_scalar!("select cancelled_by from reservations")
            .fetch_one(&pool)
            .await?;
        assert_eq!(cancelled_by, Some(user.id));

        Ok(())
    }

    #[sqlx::test]
    async fn waiting_member_is_placed_ahead_of_waiting_guest(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 1).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        create_reservation(&pool, &location, now, &user_1, date, 18, None)
            .await
            .unwrap();
        create_reservation(&pool, &location, now, &user_2, date, 20, None)
            .await
            .unwrap();
        assert_eq!(
            create_reservation(&pool, &location, now, &user_2, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: true })
        );
        assert_eq!(
            create_reservation(&pool, &location, now, &user_3, date, 18, None).await,
            Ok(ReservationSuccess::InWaiting { as_guest: false })
        );

        let history = fetch_history(&pool).await?;
        // The guest joined first, the member who came later still goes ahead of them
        assert_eq!(
            history[2],
            HistoryRow {
                event: "created".to_string(),
                user_id: user_2.id,
                hour: 18,
                in_waiting: true,
                waiting_position: Some(1),
                changed_by: Some(user_2.id),
            }
        );
        assert_eq!(
            history[3],
            row("created", user_3.id, true, Some(1), Some(user_3.id))
        );

        Ok(())
    }
    #[sqlx::test]
    async fn cleanup_records_expired_waiting_and_skips_cancelled(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        let (location, user_1, user_2, user_3) = setup(&pool, 1, 0).await?;
        let now = datetime!(2024-07-11 10:00:00 +00:00:00);
        let date = date!(2024 - 07 - 11);

        for user in [&user_1, &user_2, &user_3] {
            create_reservation(&pool, &location, now, user, date, 18, None)
                .await
                .unwrap();
        }
        let tx = pool.begin().await?;
        assert!(cancel_reservation(tx, &location, date, 18, user_3.id, None, user_3.id).await?);

        assert_eq!(delete_expired_waiting(&pool, date, 18).await?, 2);

        let history = fetch_history(&pool).await?;
        assert_eq!(
            history[3..],
            [
                row("cancelled", user_3.id, true, None, Some(user_3.id)),
                row("expired", user_2.id, true, None, None),
            ]
        );

        Ok(())
    }

    #[sqlx::test]
    async fn wiping_a_future_day_records_deleted(pool: SqlitePool) -> sqlx::Result<()> {
        let (location, user, _, _) = setup(&pool, 1, 0).await?;
        let date = date!(2099 - 01 - 01);

        query!(
            "insert into reservations (user_id, location, date, hour, in_waiting) values ($1, $2, $3, 18, true)",
            user.id,
            location.id,
            date
        )
        .execute(&pool)
        .await?;

        UserReservation::delete_on_day(&pool, date, None).await?;

        assert_eq!(
            fetch_history(&pool)
                .await?
                .last()
                .map(|row| row.event.as_str()),
            Some("deleted")
        );

        Ok(())
    }
    #[test]
    fn history_retention_covers_last_90_days() {
        let today = date!(2026 - 09 - 30);
        assert!(is_within_history_retention(today, today));
        assert!(is_within_history_retention(date!(2026 - 07 - 02), today));
        assert!(!is_within_history_retention(date!(2026 - 07 - 01), today));
    }
}
