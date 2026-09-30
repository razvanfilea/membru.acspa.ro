use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::response::hx_notify_oob;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations, format_amount};
use crate::model::user::UserDetails;
use crate::utils::date_formats::DateFormatExt;
use crate::utils::dates::{YearMonth, YearMonthIter};
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use time::{Date, Month};

#[derive(Debug, Clone, PartialEq)]
enum MonthStatus {
    Paid(String),  // Notes
    Break(String), // Reason
    Unpaid,
    NotJoined,
    Future,
}

#[derive(Debug, Clone)]
struct MonthStatusView {
    month: Month,
    status: MonthStatus,
}

impl MonthStatusView {
    fn month_number(&self) -> u8 {
        self.month as u8
    }
}

#[derive(Template)]
#[template(path = "components/payments_status_grid.html")]
pub struct StatusGridTemplate {
    member: UserDetails,
    current_year: i32,
    selected_year: i32,
    months_status_view: Vec<MonthStatusView>,
    admin_payments_status_grid: bool,
    /// Formatted amount allocated to `selected_year`, only shown to admins
    total_paid: String,
    /// Payments covering a month of `selected_year`, only listed for admins
    year_payments: Vec<PaymentWithAllocations>,
    paid_months_json: String,
    oob: bool,
}

impl StatusGridTemplate {
    pub(super) fn new(
        member: UserDetails,
        year: i32,
        admin_payments_status_grid: bool,
        payments: Vec<PaymentWithAllocations>,
        breaks: &[PaymentBreak],
    ) -> Self {
        let months_status_view = calculate_year_status(year, &member, &payments, breaks);
        let (total_paid, paid_months_json, year_payments) = if admin_payments_status_grid {
            (
                format_amount(calculate_total_paid_for_year(&payments, year)),
                paid_months_json(&payments, breaks),
                payments_for_year(payments, year),
            )
        } else {
            Default::default()
        };

        Self {
            member,
            current_year: local_date().year(),
            selected_year: year,
            months_status_view,
            admin_payments_status_grid,
            total_paid,
            year_payments,
            paid_months_json,
            oob: false,
        }
    }

    pub async fn fetch(
        pool: &SqlitePool,
        member: UserDetails,
        year: i32,
        admin_payments_status_grid: bool,
    ) -> HttpResult<Self> {
        let payments = PaymentWithAllocations::fetch_for_user(pool, member.id).await?;
        let breaks = PaymentBreak::fetch_for_user(pool, member.id).await?;
        Ok(Self::new(
            member,
            year,
            admin_payments_status_grid,
            payments,
            &breaks,
        ))
    }
}

pub async fn build_status_grid_response(
    pool: &SqlitePool,
    member: UserDetails,
    year: i32,
    admin_payments_status_grid: bool,
) -> HttpResult {
    StatusGridTemplate::fetch(pool, member, year, admin_payments_status_grid)
        .await?
        .try_into_response()
}

#[derive(Template)]
#[template(path = "admin/members/breaks_section.html")]
struct BreaksSectionTemplate {
    breaks: Vec<PaymentBreak>,
    oob: bool,
}

/// Event the member view listens to, to close the payment and break modals
const PAYMENTS_CHANGED_EVENT: &str = "paymentsChanged";

pub(super) fn view_year_or_current(view_year: Option<i32>) -> i32 {
    view_year.unwrap_or_else(|| local_date().year())
}

#[derive(Deserialize)]
pub(super) struct ViewYear {
    view_year: Option<i32>,
}

impl ViewYear {
    pub(super) fn year(&self) -> i32 {
        view_year_or_current(self.view_year)
    }
}

pub(super) async fn payments_changed_response(
    pool: &SqlitePool,
    member_id: i64,
    year: i32,
    message: &str,
) -> HttpResult {
    let member = UserDetails::fetch(pool, member_id).await?;
    let payments = PaymentWithAllocations::fetch_for_user(pool, member_id).await?;
    let breaks = PaymentBreak::fetch_for_user(pool, member_id).await?;

    let mut status_grid = StatusGridTemplate::new(member, year, true, payments, &breaks);
    status_grid.oob = true;
    let breaks_section = BreaksSectionTemplate { breaks, oob: true };

    let response = hx_notify_oob(status_grid.render()? + &breaks_section.render()?, message)?;
    Ok(([("HX-Trigger", PAYMENTS_CHANGED_EVENT)], response).into_response())
}

fn paid_months_json(payments: &[PaymentWithAllocations], breaks: &[PaymentBreak]) -> String {
    let mut months: HashSet<String> = payments
        .iter()
        .flat_map(|p| p.allocations.iter())
        .map(|alloc| format!("{}-{:04}", alloc.month as u8, alloc.year))
        .collect();

    for br in breaks {
        let start = YearMonth::from(br.start_date);
        let end = YearMonth::from(br.end_date);

        for ym in YearMonthIter::new(start, end) {
            months.insert(format!("{}-{:04}", ym.month as u8, ym.year));
        }
    }

    serde_json::to_string(&months).expect("Failed to serialize")
}

/// Payments with at least one month allocated to `year`, a payment spanning two years is in both
fn payments_for_year(
    payments: Vec<PaymentWithAllocations>,
    year: i32,
) -> Vec<PaymentWithAllocations> {
    payments
        .into_iter()
        .filter(|payment| payment.allocations.iter().any(|alloc| alloc.year == year))
        .collect()
}

fn calculate_total_paid_for_year(payments: &[PaymentWithAllocations], year: i32) -> i64 {
    payments
        .iter()
        .map(|p| {
            let months_in_year = p.allocations.iter().filter(|a| a.year == year).count();
            let total_months = p.allocations.len();
            if total_months == 0 {
                0
            } else {
                p.amount * months_in_year as i64 / total_months as i64
            }
        })
        .sum()
}

fn calculate_year_status(
    year: i32,
    member: &UserDetails,
    payments: &[PaymentWithAllocations],
    breaks: &[PaymentBreak],
) -> Vec<MonthStatusView> {
    let current_date = local_date();

    YearMonthIter::for_year(year)
        .map(|year_month| {
            let month_start = year_month.to_date();
            let month = year_month.month;

            let member_start_month = YearMonth::from(member.member_since).to_date();

            if month_start < member_start_month {
                return MonthStatusView {
                    month,
                    status: MonthStatus::NotJoined,
                };
            }

            let is_paid = payments.iter().find(|p| {
                p.allocations
                    .iter()
                    .any(|a| a.year == year && a.month == month)
            });

            if let Some(paid) = is_paid {
                return MonthStatusView {
                    month,
                    status: MonthStatus::Paid(paid.notes.clone().unwrap_or_default()),
                };
            }

            let is_break = breaks.iter().find(|b| {
                // Breaks are stored as 1st of month.
                month_start >= b.start_date && month_start <= b.end_date
            });

            if let Some(brk) = is_break {
                return MonthStatusView {
                    month,
                    status: MonthStatus::Break(brk.reason.clone().unwrap_or_default()),
                };
            }

            let current_month_start =
                Date::from_calendar_date(current_date.year(), current_date.month(), 1).unwrap();

            if month_start > current_month_start {
                return MonthStatusView {
                    month,
                    status: MonthStatus::Future,
                };
            }

            MonthStatusView {
                month,
                status: MonthStatus::Unpaid,
            }
        })
        .collect()
}

pub(super) async fn payments_status_partial(
    State(state): State<AppState>,
    Path((user_id, year)): Path<(i64, i32)>,
) -> HttpResult {
    let member = UserDetails::fetch(&state.read_pool, user_id).await?;
    build_status_grid_response(&state.read_pool, member, year, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Month, macros::date};

    fn make_member(member_since: Date) -> UserDetails {
        UserDetails {
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

        for month in &result[..5] {
            assert_eq!(month.status, MonthStatus::NotJoined);
        }
        assert_ne!(result[5].status, MonthStatus::NotJoined);
    }

    #[test]
    fn returns_paid_when_payment_allocation_exists() {
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::March, Month::April], Some("Cash"));
        let result = calculate_year_status(2020, &member, &[payment], &[]);

        assert_eq!(result[2].status, MonthStatus::Paid("Cash".to_string()));
        assert_eq!(result[3].status, MonthStatus::Paid("Cash".to_string()));
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

        assert_eq!(result[2].status, MonthStatus::Break("Medical".to_string()));
        assert_eq!(result[3].status, MonthStatus::Break("Medical".to_string()));
        assert_eq!(result[4].status, MonthStatus::Break("Medical".to_string()));
    }

    #[test]
    fn returns_unpaid_for_past_months_without_payment_or_break() {
        let member = make_member(date!(2020 - 01 - 01));
        let result = calculate_year_status(2020, &member, &[], &[]);

        for status in &result {
            assert_eq!(status.status, MonthStatus::Unpaid);
        }
    }

    #[test]
    fn returns_future_for_months_after_current_date() {
        let member = make_member(date!(2099 - 01 - 01));
        let result = calculate_year_status(2099, &member, &[], &[]);

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

        assert_eq!(result[0].status, MonthStatus::NotJoined);
        assert_eq!(result[1].status, MonthStatus::NotJoined);
        assert_eq!(result[2].status, MonthStatus::NotJoined);

        assert_eq!(result[3].status, MonthStatus::Paid(String::new()));
        assert_eq!(result[4].status, MonthStatus::Paid(String::new()));

        assert_eq!(result[5].status, MonthStatus::Unpaid);

        assert_eq!(result[6].status, MonthStatus::Break(String::new()));
        assert_eq!(result[7].status, MonthStatus::Break(String::new()));

        for month in &result[8..] {
            assert_eq!(month.status, MonthStatus::Unpaid);
        }
    }

    #[test]
    fn payment_takes_precedence_over_break() {
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::March], Some("Paid"));
        let brk = make_break(date!(2020 - 03 - 01), date!(2020 - 03 - 01), Some("Break"));

        let result = calculate_year_status(2020, &member, &[payment], &[brk]);

        assert_eq!(result[2].status, MonthStatus::Paid("Paid".to_string()));
    }

    #[test]
    fn months_are_correct() {
        let member = make_member(date!(2020 - 01 - 01));
        let result = calculate_year_status(2020, &member, &[], &[]);

        assert_eq!(result[0].month, Month::January);
        assert_eq!(result[5].month, Month::June);
        assert_eq!(result[11].month, Month::December);
    }

    #[test]
    fn payment_covers_first_and_last_month_of_year() {
        let member = make_member(date!(2020 - 01 - 01));
        let payment = make_payment(2020, &[Month::January, Month::December], None);
        let result = calculate_year_status(2020, &member, &[payment], &[]);

        assert_eq!(result[0].status, MonthStatus::Paid(String::new()));
        assert_eq!(result[11].status, MonthStatus::Paid(String::new()));
        assert_eq!(result[6].status, MonthStatus::Unpaid);
    }
}
