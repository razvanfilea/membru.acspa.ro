use crate::model::global_vars::GlobalVars;
use crate::model::user::User;
use crate::utils::dates::{YearMonth, YearMonthIter};
use crate::utils::{date_formats, local_date};
use itertools::Itertools;
use sqlx::{SqliteExecutor, SqlitePool, query_as};
use std::collections::HashSet;
use time::{Date, Month};

const USER_ROLLS_TO_SKIP: &[i64] = &[1, 2, 5, 7];

pub struct DebtorItem {
    pub member: User,
    pub unpaid_months: Vec<&'static str>,
}

pub async fn compute_debtors(
    pool: &SqlitePool,
    selected_year: i32,
) -> sqlx::Result<Vec<DebtorItem>> {
    let mut conn = pool.acquire().await?;
    let current_date = local_date();
    let users = query_as!(
        User,
        "select * from users_with_role where is_active = true and admin_panel_access = false order by name"
    )
    .fetch_all(conn.as_mut())
    .await?;

    #[derive(PartialEq, Eq, Hash)]
    struct PaidMonth {
        user_id: i64,
        month: u8,
    }
    let paid_months: HashSet<_> = query_as!(
        PaidMonth,
        r#"select p.user_id, pa.month as "month!: u8"
           from payment_allocations pa
           join payments p on p.id = pa.payment_id
           where pa.year = $1"#,
        selected_year
    )
    .fetch_all(conn.as_mut())
    .await?
    .into_iter()
    .collect();

    // C. Fetch Breaks
    let year_start = Date::from_calendar_date(selected_year, Month::January, 1).unwrap();
    let year_end = Date::from_calendar_date(selected_year, Month::December, 31).unwrap();

    struct BreakRow {
        user_id: i64,
        start_date: Date,
        end_date: Date,
    }
    let breaks_lookup = query_as!(
        BreakRow,
        "select user_id, start_date, end_date from payment_breaks where start_date <= $2 and end_date >= $1",
        year_start, year_end
    )
        .fetch_all(conn.as_mut())
        .await?
        .into_iter()
        .into_group_map_by(|br| br.user_id);

    // D. Calculate Unpaid Months
    let current_month_start = YearMonth::from(current_date).to_date();
    let year_months = YearMonthIter::for_year(selected_year);

    let debtors = users
        .into_iter()
        .filter(|member| !USER_ROLLS_TO_SKIP.contains(&member.role_id))
        .filter_map(|member| {
            // We assume member_since implies they owe for that month.
            let join_month_start = member.member_since.replace_day(1).ok()?;

            let member_breaks = breaks_lookup
                .get(&member.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let unpaid_months: Vec<&'static str> = year_months
                .clone()
                .filter(|year_month| {
                    let date = year_month.to_date();
                    // Filter out Future and Pre-Join dates
                    if date > current_month_start || date < join_month_start {
                        return false;
                    }

                    // Filter out Paid months
                    if paid_months.contains(&PaidMonth {
                        user_id: member.id,
                        month: date.month() as u8,
                    }) {
                        return false;
                    }

                    // Filter out Breaks
                    if member_breaks
                        .iter()
                        .any(|b| (b.start_date..=b.end_date).contains(&date))
                    {
                        return false;
                    }

                    true
                })
                .map(|date| date_formats::month_as_str(&date.month))
                .collect();

            if !unpaid_months.is_empty() {
                Some(DebtorItem {
                    member,
                    unpaid_months,
                })
            } else {
                None
            }
        })
        .sorted_by(|a, b| b.unpaid_months.len().cmp(&a.unpaid_months.len()))
        .collect();

    Ok(debtors)
}

/// Checks if a user has a valid payment allocation or break for a specific year/month.
async fn is_month_covered(
    executor: impl SqliteExecutor<'_>,
    user_id: i64,
    year: i32,
    month: Month,
) -> sqlx::Result<bool> {
    // We construct the 1st of the month to check against break ranges
    let first_of_month = Date::from_calendar_date(year, month, 1).unwrap();

    let month = month as u8;
    let count = sqlx::query_scalar!(
        r#"
        select count(*) from (
            -- check for payment allocation
            select 1 from payment_allocations pa
            join payments p on pa.payment_id = p.id
            where p.user_id = $1 and pa.year = $2 and pa.month = $3

            union

            -- check for payment break
            -- breaks store start/end as dates (1st of month).
            -- a break covers this month if the 1st of the month is within the range.
            select 1 from payment_breaks pb
            where pb.user_id = $1 and pb.start_date <= $4 and pb.end_date >= $4
        )
        "#,
        user_id,
        year,
        month,
        first_of_month
    )
    .fetch_one(executor)
    .await?;

    Ok(count > 0)
}

pub async fn check_user_has_paid(pool: &SqlitePool, user: &User) -> sqlx::Result<bool> {
    if user.admin_panel_access || USER_ROLLS_TO_SKIP.contains(&user.role_id) {
        return Ok(true);
    }
    let mut tx = pool.begin().await?;
    let global_vars = GlobalVars::fetch(tx.as_mut()).await?;
    if !global_vars.check_payments {
        return Ok(true);
    }

    let current_ym = YearMonth::from(local_date());
    let start_ym = current_ym.prev().prev();

    // Check current month + previous 2 months
    for ym in YearMonthIter::new(start_ym, current_ym) {
        if is_month_covered(tx.as_mut(), user.id, ym.year, ym.month).await? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{SqlitePool, query};

    async fn setup_test_data(pool: &SqlitePool) -> sqlx::Result<()> {
        // Create test role for regular members (role 1 'Admin' already exists from migrations)
        // Role 100: Regular member (should be checked for debtors)
        query!(
            r#"
            INSERT INTO user_roles (id, name, reservations, guest_reservations, admin_panel_access)
            VALUES (100, 'Member', 1, 0, FALSE)
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn insert_user(
        pool: &SqlitePool,
        id: i64,
        name: &str,
        role_id: i64,
        member_since: &str,
    ) -> sqlx::Result<()> {
        let email = format!("{}@test.com", name);
        query!(
            r#"
            INSERT INTO users (id, email, name, password_hash, role_id, has_key, birthday, member_since, is_active)
            VALUES ($1, $2, $3, '', $4, FALSE, '2000-01-01', $5, TRUE)
            "#,
            id,
            email,
            name,
            role_id,
            member_since
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn insert_payment(
        pool: &SqlitePool,
        user_id: i64,
        year: i32,
        months: &[u8],
    ) -> sqlx::Result<()> {
        // Insert payment
        let payment_id = query!(
            r#"
            INSERT INTO payments (user_id, amount, payment_date, created_by)
            VALUES ($1, 5000, '2020-01-15', $1)
            RETURNING id
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?
        .id;

        // Insert allocations
        for &month in months {
            query!(
                "INSERT INTO payment_allocations (payment_id, year, month) VALUES ($1, $2, $3)",
                payment_id,
                year,
                month
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    async fn insert_break(
        pool: &SqlitePool,
        user_id: i64,
        start: &str,
        end: &str,
    ) -> sqlx::Result<()> {
        query!(
            r#"
            INSERT INTO payment_breaks (user_id, start_date, end_date, created_by)
            VALUES ($1, $2, $3, $1)
            "#,
            user_id,
            start,
            end
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn returns_empty_when_all_users_paid(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;
        // User joined Jan 2020, paid all months
        insert_user(&pool, 1, "Paid User", 100, "2020-01-01").await?;
        insert_payment(&pool, 1, 2020, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]).await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert!(debtors.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn returns_users_with_unpaid_months_sorted_by_count(
        pool: SqlitePool,
    ) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User A: joined Jan 2020, paid 6 months (6 unpaid)
        insert_user(&pool, 1, "A User", 100, "2020-01-01").await?;
        insert_payment(&pool, 1, 2020, &[1, 2, 3, 4, 5, 6]).await?;

        // User B: joined Jan 2020, paid 9 months (3 unpaid)
        insert_user(&pool, 2, "B User", 100, "2020-01-01").await?;
        insert_payment(&pool, 2, 2020, &[1, 2, 3, 4, 5, 6, 7, 8, 9]).await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 2);

        // Should be sorted by unpaid count (descending)
        assert_eq!(debtors[0].member.name, "A User");
        assert_eq!(debtors[0].unpaid_months.len(), 6);
        assert_eq!(debtors[1].member.name, "B User");
        assert_eq!(debtors[1].unpaid_months.len(), 3);
        Ok(())
    }

    #[sqlx::test]
    async fn skips_roles_in_user_rolls_to_skip(pool: SqlitePool) -> sqlx::Result<()> {
        // Create a role that's in USER_ROLLS_TO_SKIP but has admin_panel_access = FALSE
        // This ensures we're testing the USER_ROLLS_TO_SKIP logic, not the SQL filter
        query!(
            r#"
            INSERT INTO user_roles (id, name, reservations, guest_reservations, admin_panel_access)
            VALUES (100, 'Member', 1, 0, FALSE),
                   (5, 'SkippedRole', 1, 0, FALSE)
            "#
        )
        .execute(&pool)
        .await?;

        // Regular member with unpaid months
        insert_user(&pool, 1, "Regular", 100, "2020-01-01").await?;

        // User with role 5 (in USER_ROLLS_TO_SKIP) but NOT admin - tests the actual skip logic
        insert_user(&pool, 2, "SkippedUser", 5, "2020-01-01").await?;

        let debtors = compute_debtors(&pool, 2020).await?;

        // Only the regular member should appear (role 5 is skipped by USER_ROLLS_TO_SKIP)
        assert_eq!(debtors.len(), 1);
        assert_eq!(debtors[0].member.name, "Regular");
        Ok(())
    }

    #[sqlx::test]
    async fn respects_member_since_date(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User joined in July 2020
        insert_user(&pool, 1, "Late Joiner", 100, "2020-07-01").await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 1);

        // Should only owe for Jul-Dec (6 months), not Jan-Jun
        assert_eq!(debtors[0].unpaid_months.len(), 6);

        // Verify the months are correct (Romanian names)
        assert!(debtors[0].unpaid_months.contains(&"Iulie"));
        assert!(debtors[0].unpaid_months.contains(&"Decembrie"));
        assert!(!debtors[0].unpaid_months.contains(&"Ianuarie"));
        Ok(())
    }

    #[sqlx::test]
    async fn respects_payment_breaks(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User joined Jan 2020, has break Mar-May
        insert_user(&pool, 1, "Break User", 100, "2020-01-01").await?;
        insert_break(&pool, 1, "2020-03-01", "2020-05-01").await?;

        // Pay Jan-Feb only
        insert_payment(&pool, 1, 2020, &[1, 2]).await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 1);

        // Should owe for Jun-Dec (7 months), not Jan-Feb (paid) or Mar-May (break)
        assert_eq!(debtors[0].unpaid_months.len(), 7);
        assert!(debtors[0].unpaid_months.contains(&"Iunie"));
        assert!(!debtors[0].unpaid_months.contains(&"Martie"));
        Ok(())
    }

    #[sqlx::test]
    async fn handles_user_with_no_payments(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User joined Jan 2020, no payments
        insert_user(&pool, 1, "No Payments", 100, "2020-01-01").await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 1);

        // Should owe for all 12 months
        assert_eq!(debtors[0].unpaid_months.len(), 12);
        Ok(())
    }

    #[sqlx::test]
    async fn user_joined_in_selected_year(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User joined Oct 2020
        insert_user(&pool, 1, "Oct Joiner", 100, "2020-10-15").await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 1);

        // Should owe for Oct-Dec (3 months)
        assert_eq!(debtors[0].unpaid_months.len(), 3);
        assert!(debtors[0].unpaid_months.contains(&"Octombrie"));
        assert!(debtors[0].unpaid_months.contains(&"Noiembrie"));
        assert!(debtors[0].unpaid_months.contains(&"Decembrie"));
        Ok(())
    }

    #[sqlx::test]
    async fn break_spanning_multiple_months(pool: SqlitePool) -> sqlx::Result<()> {
        setup_test_data(&pool).await?;

        // User with 6-month break
        insert_user(&pool, 1, "Long Break", 100, "2020-01-01").await?;
        insert_break(&pool, 1, "2020-01-01", "2020-06-01").await?;

        let debtors = compute_debtors(&pool, 2020).await?;
        assert_eq!(debtors.len(), 1);

        // Should owe for Jul-Dec only (6 months)
        assert_eq!(debtors[0].unpaid_months.len(), 6);
        assert!(debtors[0].unpaid_months.contains(&"Iulie"));
        assert!(!debtors[0].unpaid_months.contains(&"Ianuarie"));
        Ok(())
    }
}
