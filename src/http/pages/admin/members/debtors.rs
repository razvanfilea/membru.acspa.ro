use crate::model::user::User;
use crate::utils::dates::{YearMonth, YearMonthIter};
use crate::utils::queries::get_global_vars;
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
    let global_vars = get_global_vars(tx.as_mut()).await?;
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
