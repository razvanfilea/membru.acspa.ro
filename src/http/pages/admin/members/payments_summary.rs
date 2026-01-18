use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::user::User;
use crate::utils::dates::{YearMonth, YearMonthIter};
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, State};
use sqlx::SqlitePool;
use time::Date;

#[derive(Debug, Clone, PartialEq)]
pub enum MonthStatus {
    Paid(String),  // Notes
    Break(String), // Reason
    Unpaid,
    NotJoined,
    Future,
}

#[derive(Debug, Clone)]
pub struct MonthStatusView {
    pub month_name: &'static str,
    pub status: MonthStatus,
}

#[derive(Template)]
#[template(path = "components/payments_status_grid.html")]
pub struct StatusGridTemplate {
    pub user: User,
    pub member: User,
    pub current_year: i32,
    pub selected_year: i32,
    pub months_status_view: Vec<MonthStatusView>,
}

pub async fn build_status_grid_response(
    pool: &SqlitePool,
    user: User,
    member: User,
    year: i32,
) -> HttpResult {
    let payments = PaymentWithAllocations::fetch_for_user(pool, member.id).await?;
    let breaks = PaymentBreak::fetch_for_user(pool, member.id).await?;
    let current_year = local_date().year();
    let months_status_view = calculate_year_status(year, &member, &payments, &breaks);

    StatusGridTemplate {
        user,
        member,
        current_year,
        selected_year: year,
        months_status_view,
    }
    .try_into_response()
}

pub fn calculate_year_status(
    year: i32,
    member: &User,
    payments: &[PaymentWithAllocations],
    breaks: &[PaymentBreak],
) -> Vec<MonthStatusView> {
    let current_date = local_date();

    YearMonthIter::for_year(year)
        .map(|year_month| {
            let month_start = year_month.to_date();
            let month_name = date_formats::month_as_str(&year_month.month);

            // 1. Check if before member joined (approximate to month)
            let member_start_month = YearMonth::from(member.member_since).to_date();

            if month_start < member_start_month {
                return MonthStatusView {
                    month_name,
                    status: MonthStatus::NotJoined,
                };
            }

            // 2. Check if Paid
            let is_paid = payments.iter().find(|p| {
                p.allocations
                    .iter()
                    .any(|a| a.year == year && a.month == year_month.month)
            });

            if let Some(paid) = is_paid {
                return MonthStatusView {
                    month_name,
                    status: MonthStatus::Paid(paid.notes.clone().unwrap_or_default()),
                };
            }

            // 3. Check if Break
            let is_break = breaks.iter().find(|b| {
                // Check if this month (e.g. 2024-05-01) is within break start..=end
                // Breaks are stored as 1st of month.
                month_start >= b.start_date && month_start <= b.end_date
            });

            if let Some(brk) = is_break {
                return MonthStatusView {
                    month_name,
                    status: MonthStatus::Break(brk.reason.clone().unwrap_or_default()),
                };
            }

            // 4. Check Future vs Unpaid
            // If the month is in the future compared to now
            let current_month_start =
                Date::from_calendar_date(current_date.year(), current_date.month(), 1).unwrap();

            if month_start > current_month_start {
                return MonthStatusView {
                    month_name,
                    status: MonthStatus::Future,
                };
            }

            MonthStatusView {
                month_name,
                status: MonthStatus::Unpaid,
            }
        })
        .collect()
}

pub async fn payments_status_partial(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Path((user_id, year)): Path<(i64, i32)>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let member = User::fetch(&state.read_pool, user_id).await?;
    build_status_grid_response(&state.read_pool, user, member, year).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Month, macros::date};

    fn make_member(member_since: Date) -> User {
        User {
            member_since,
            ..Default::default()
        }
    }

    fn make_payment(year: i32, months: &[Month], notes: Option<&str>) -> PaymentWithAllocations {
        PaymentWithAllocations {
            id: 1,
            amount: 5000,
            payment_date: date!(2020 - 01 - 15),
            notes: notes.map(|s| s.to_string()),
            allocations: months.iter().map(|&m| YearMonth::new(year, m)).collect(),
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            created_by: 1,
            created_by_name: "Admin".to_string(),
        }
    }

    fn make_break(start: Date, end: Date, reason: Option<&str>) -> PaymentBreak {
        PaymentBreak {
            id: 1,
            user_id: 1,
            start_date: start,
            end_date: end,
            reason: reason.map(|s| s.to_string()),
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            created_by: 1,
            created_by_name: "Admin".to_string(),
        }
    }

    #[test]
    fn returns_not_joined_for_months_before_member_since() {
        let member = make_member(date!(2020 - 06 - 15));
        let result = calculate_year_status(2020, &member, &[], &[]);

        // January through May should be NotJoined
        for i in 0..5 {
            assert_eq!(result[i].status, MonthStatus::NotJoined);
        }
        // June onwards should be something else (Unpaid in this case)
        assert_ne!(result[5].status, MonthStatus::NotJoined);
    }

    #[test]
    fn returns_paid_when_payment_allocation_exists() {
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::March, Month::April], Some("Cash"));
        let result = calculate_year_status(2020, &member, &[payment], &[]);

        assert_eq!(result[2].status, MonthStatus::Paid("Cash".to_string())); // March
        assert_eq!(result[3].status, MonthStatus::Paid("Cash".to_string())); // April
    }

    #[test]
    fn returns_break_when_month_within_break_period() {
        let member = make_member(date!(2020 - 01 - 01));
        let brk = make_break(
            date!(2020 - 03 - 01),
            date!(2020 - 05 - 01),
            Some("Medical"),
        );
        let result = calculate_year_status(2020, &member, &[], &[brk]);

        assert_eq!(result[2].status, MonthStatus::Break("Medical".to_string())); // March
        assert_eq!(result[3].status, MonthStatus::Break("Medical".to_string())); // April
        assert_eq!(result[4].status, MonthStatus::Break("Medical".to_string())); // May
    }

    #[test]
    fn returns_unpaid_for_past_months_without_payment_or_break() {
        let member = make_member(date!(2020 - 01 - 01));
        let result = calculate_year_status(2020, &member, &[], &[]);

        // All months in 2020 should be Unpaid (since it's in the past)
        for status in &result {
            assert_eq!(status.status, MonthStatus::Unpaid);
        }
    }

    #[test]
    fn returns_future_for_months_after_current_date() {
        let member = make_member(date!(2099 - 01 - 01));
        let result = calculate_year_status(2099, &member, &[], &[]);

        // All months in 2099 should be Future (except possibly current month if we're in 2099)
        // Since we're definitely not in 2099, all should be Future
        for status in &result {
            assert_eq!(status.status, MonthStatus::Future);
        }
    }

    #[test]
    fn mixed_scenario_user_joined_mid_year() {
        let member = make_member(date!(2020 - 04 - 01));
        let payment = make_payment(2020, &[Month::April, Month::May], None);
        let brk = make_break(date!(2020 - 07 - 01), date!(2020 - 08 - 01), None);

        let result = calculate_year_status(2020, &member, &[payment], &[brk]);

        // Jan-Mar: NotJoined
        assert_eq!(result[0].status, MonthStatus::NotJoined);
        assert_eq!(result[1].status, MonthStatus::NotJoined);
        assert_eq!(result[2].status, MonthStatus::NotJoined);

        // Apr-May: Paid
        assert_eq!(result[3].status, MonthStatus::Paid(String::new()));
        assert_eq!(result[4].status, MonthStatus::Paid(String::new()));

        // Jun: Unpaid
        assert_eq!(result[5].status, MonthStatus::Unpaid);

        // Jul-Aug: Break
        assert_eq!(result[6].status, MonthStatus::Break(String::new()));
        assert_eq!(result[7].status, MonthStatus::Break(String::new()));

        // Sep-Dec: Unpaid
        for i in 8..12 {
            assert_eq!(result[i].status, MonthStatus::Unpaid);
        }
    }

    #[test]
    fn payment_takes_precedence_over_break() {
        // If a month has both payment and break, payment should win (checked first)
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::March], Some("Paid"));
        let brk = make_break(date!(2020 - 03 - 01), date!(2020 - 03 - 01), Some("Break"));

        let result = calculate_year_status(2020, &member, &[payment], &[brk]);

        assert_eq!(result[2].status, MonthStatus::Paid("Paid".to_string()));
    }

    #[test]
    fn month_names_are_in_romanian() {
        let member = make_member(date!(2020 - 01 - 01));
        let result = calculate_year_status(2020, &member, &[], &[]);

        assert_eq!(result[0].month_name, "Ianuarie");
        assert_eq!(result[5].month_name, "Iunie");
        assert_eq!(result[11].month_name, "Decembrie");
    }

    #[test]
    fn payment_covers_first_and_last_month_of_year() {
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::January, Month::December], None);
        let result = calculate_year_status(2020, &member, &[payment], &[]);

        assert_eq!(result[0].status, MonthStatus::Paid(String::new())); // January
        assert_eq!(result[11].status, MonthStatus::Paid(String::new())); // December
        // Middle months should be Unpaid
        assert_eq!(result[6].status, MonthStatus::Unpaid); // July
    }
}
