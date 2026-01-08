use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::admin::members::breaks::get_user_payment_breaks;
use crate::http::pages::admin::members::payments::get_user_payments;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::user::User;
use crate::utils::queries::get_user;
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, State};
use time::{Date, Month};

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

pub fn calculate_year_status(
    year: i32,
    member: &User,
    payments: &[PaymentWithAllocations],
    breaks: &[PaymentBreak],
) -> Vec<MonthStatusView> {
    let current_date = local_date();

    (1..=12)
        .map(|m| {
            let month = Month::try_from(m).unwrap();
            let month_start = Date::from_calendar_date(year, month, 1).unwrap();
            let month_name = date_formats::month_as_str(&month);

            // 1. Check if before member joined (approximate to month)
            let member_start_month = Date::from_calendar_date(
                member.member_since.year(),
                member.member_since.month(),
                1,
            )
            .unwrap();

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
                    .any(|a| a.year == year && a.month == month)
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
    State(state): State<AppState>,
    Path((user_id, year)): Path<(i64, i32)>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/status_grid_partial.html")]
    struct StatusGridTemplate {
        member: User,
        current_year: i32,
        selected_year: i32,
        months_status_view: Vec<MonthStatusView>,
    }

    let member = get_user(&state.read_pool, user_id).await?;
    let payments = get_user_payments(&state.read_pool, user_id).await?;
    let breaks = get_user_payment_breaks(&state.read_pool, user_id).await?;

    let current_year = local_date().year();

    let months = calculate_year_status(year, &member, &payments, &breaks);

    StatusGridTemplate {
        member,
        current_year,
        selected_year: year,
        months_status_view: months,
    }
    .try_into_response()
}
