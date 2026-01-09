use crate::model::user::User;
use crate::utils::queries::YearMonth;
use crate::utils::{date_formats, local_date};
use itertools::Itertools;
use sqlx::{SqlitePool, query_as};
use std::collections::HashSet;
use time::{Date, Month};

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
        "select * from users_with_role where is_active = true order by name"
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
        "SELECT user_id, start_date, end_date FROM payment_breaks WHERE start_date <= $2 AND end_date >= $1",
        year_start, year_end
    )
        .fetch_all(conn.as_mut())
        .await?
        .into_iter()
        .into_group_map_by(|br| br.user_id);

    // D. Calculate Unpaid Months
    let current_month_start = YearMonth::from(current_date).to_date();
    let year_months: Vec<_> = (1..=12)
        .map(|m| {
            let month = Month::try_from(m).unwrap();
            YearMonth::new(selected_year, month).to_date()
        })
        .collect();

    let debtors = users
        .into_iter()
        .filter_map(|member| {
            // We assume member_since implies they owe for that month.
            let join_month_start = member.member_since.replace_day(1).ok()?;

            let member_breaks = breaks_lookup
                .get(&member.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let unpaid_months: Vec<&'static str> = year_months
                .iter()
                .filter(|date| {
                    let date = **date;
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
                .map(|date| date_formats::month_as_str(&date.month()))
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
