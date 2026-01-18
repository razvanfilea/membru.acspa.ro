use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::model::payment_context::PaymentContext;
use crate::model::user::User;
use crate::utils::dates::YearMonth;
use crate::utils::local_date;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::query;
use std::fmt;
use time::{Date, Month};
use tracing::info;

#[derive(Deserialize, Debug)]
pub struct NewPayment {
    amount: f64,        // From input type="number" step="0.01"
    payment_date: Date, // From input type="date"
    months: String,     // From checkboxes (format: "M-YYYY")
    notes: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum PaymentValidationError {
    InvalidAmount,
    NoValidMonths,
    MonthAlreadyPaid(YearMonth),
    MonthInBreak(YearMonth),
}

impl fmt::Display for PaymentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAmount => write!(f, "Suma trebuie să fie pozitivă"),
            Self::NoValidMonths => {
                write!(f, "O plată trebuie să acopere cel puțin o lună validă")
            }
            Self::MonthAlreadyPaid(m) => {
                write!(f, "Luna {}-{} este deja plătită", m.month as u8, m.year)
            }
            Self::MonthInBreak(m) => {
                write!(f, "Luna {}-{} este marcată ca pauză", m.month as u8, m.year)
            }
        }
    }
}

fn validate_and_convert_amount(amount: f64) -> Result<i64, PaymentValidationError> {
    if amount <= 0.0 {
        return Err(PaymentValidationError::InvalidAmount);
    }
    Ok((amount * 100.0).round() as i64)
}

fn parse_month_allocations(
    months_str: &str,
    member_since: Date,
    current_year: i32,
) -> Result<Vec<YearMonth>, PaymentValidationError> {
    let valid_year_range = member_since.year()..=current_year + 1;
    let joining_month = YearMonth::from(member_since);

    let allocations: Vec<_> = months_str
        .split(',')
        .filter_map(|s| {
            let (month_str, year_str) = s.trim().split_once('-')?;

            let month = month_str
                .parse::<u8>()
                .ok()
                .and_then(|m| Month::try_from(m).ok())?;

            let year = year_str
                .parse::<i32>()
                .ok()
                .filter(|y| valid_year_range.contains(y))?;

            Some(YearMonth::new(year, month)).filter(|parsed| parsed >= &joining_month)
        })
        .collect();

    if allocations.is_empty() {
        return Err(PaymentValidationError::NoValidMonths);
    }

    Ok(allocations)
}

fn validate_allocations(
    requested: &[YearMonth],
    ctx: &PaymentContext,
) -> Result<(), PaymentValidationError> {
    for &month in requested {
        if ctx.is_month_paid(month) {
            return Err(PaymentValidationError::MonthAlreadyPaid(month));
        }
        if ctx.is_month_in_break(month) {
            return Err(PaymentValidationError::MonthInBreak(month));
        }
    }
    Ok(())
}

pub async fn add_payment(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewPayment>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let member = User::fetch(&state.read_pool, member_id).await?;

    let amount_cents =
        validate_and_convert_amount(form.amount).map_err(|e| HttpError::Message(e.to_string()))?;

    let current_year = local_date().year();
    let requested_allocations =
        parse_month_allocations(&form.months, member.member_since, current_year)
            .map_err(|e| HttpError::Message(e.to_string()))?;

    let mut tx = state.write_pool.begin().await?;

    let ctx = PaymentContext::fetch(tx.as_mut(), member_id).await?;

    validate_allocations(&requested_allocations, &ctx)
        .map_err(|e| HttpError::Message(e.to_string()))?;

    let notes = form.notes.filter(|notes| !notes.is_empty());
    let payment_id = query!(
        "insert into payments (user_id, amount, payment_date, notes, created_by)
         values ($1, $2, $3, $4, $5) returning id",
        member_id,
        amount_cents,
        form.payment_date,
        notes,
        user.id,
    )
    .fetch_one(tx.as_mut())
    .await?
    .id;

    info!(
        "Payment added: Member {} paid {:.2} RON for {} months (Req by Admin {})",
        member_id,
        form.amount,
        requested_allocations.len(),
        user.id
    );

    for requested in requested_allocations {
        let month = requested.month as u8;
        query!(
            "insert into payment_allocations (payment_id, year, month) values ($1, $2, $3)",
            payment_id,
            requested.year,
            month,
        )
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;

    Ok([("HX-Refresh", "true")].into_response())
}

pub async fn delete_payment(
    State(state): State<AppState>,
    Path(payment_id): Path<i64>,
) -> HttpResult {
    query!("delete from payments where id = $1", payment_id)
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Refresh", "true")].into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::payment::PaymentBreak;
    use time::macros::date;

    #[test]
    fn test_validate_and_convert_amount() {
        assert_eq!(validate_and_convert_amount(100.50), Ok(10050));
        assert_eq!(
            validate_and_convert_amount(0.0),
            Err(PaymentValidationError::InvalidAmount)
        );
        assert_eq!(
            validate_and_convert_amount(-50.0),
            Err(PaymentValidationError::InvalidAmount)
        );
        assert_eq!(validate_and_convert_amount(0.01), Ok(1));
        assert_eq!(validate_and_convert_amount(0.99), Ok(99));
        assert_eq!(validate_and_convert_amount(999999.99), Ok(99999999));
    }

    #[test]
    fn test_parse_month_allocations() {
        // Valid single month
        assert_eq!(
            parse_month_allocations("3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        // Valid multiple months
        assert_eq!(
            parse_month_allocations("1-2024,2-2024,3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![
                YearMonth::new(2024, Month::January),
                YearMonth::new(2024, Month::February),
                YearMonth::new(2024, Month::March),
            ])
        );

        // Invalid month format is skipped
        assert_eq!(
            parse_month_allocations("invalid,3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        // Month before member_since is skipped
        assert_eq!(
            parse_month_allocations("1-2020,6-2020", date!(2020 - 03 - 15), 2024),
            Ok(vec![YearMonth::new(2020, Month::June)])
        );

        // Year outside valid range is skipped (member_since 2020, current_year 2024, range 2020-2025)
        assert_eq!(
            parse_month_allocations("1-2019,3-2024,1-2027", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        // Empty string returns error
        assert_eq!(
            parse_month_allocations("", date!(2020 - 01 - 01), 2024),
            Err(PaymentValidationError::NoValidMonths)
        );

        // All invalid months returns error
        assert_eq!(
            parse_month_allocations("invalid,bad,nope", date!(2020 - 01 - 01), 2024),
            Err(PaymentValidationError::NoValidMonths)
        );
    }

    #[test]
    fn test_validate_allocations() {
        // All months valid returns Ok
        let ctx = PaymentContext::new(vec![], vec![]);
        let months = vec![
            YearMonth::new(2024, Month::January),
            YearMonth::new(2024, Month::February),
        ];
        assert_eq!(validate_allocations(&months, &ctx), Ok(()));

        // Already paid month returns error
        let paid_month = YearMonth::new(2024, Month::January);
        let ctx = PaymentContext::new(vec![paid_month], vec![]);
        let months = vec![paid_month, YearMonth::new(2024, Month::February)];
        assert_eq!(
            validate_allocations(&months, &ctx),
            Err(PaymentValidationError::MonthAlreadyPaid(paid_month))
        );

        // Month in break returns error
        let ctx = PaymentContext::new(
            vec![],
            vec![PaymentBreak::make_break(
                date!(2024 - 03 - 01),
                date!(2024 - 05 - 01),
            )],
        );
        let break_month = YearMonth::new(2024, Month::April);
        let months = vec![YearMonth::new(2024, Month::January), break_month];
        assert_eq!(
            validate_allocations(&months, &ctx),
            Err(PaymentValidationError::MonthInBreak(break_month))
        );

        // Multiple issues returns first error (paid month comes first)
        let paid_month = YearMonth::new(2024, Month::January);
        let break_month = YearMonth::new(2024, Month::April);
        let ctx = PaymentContext::new(
            vec![paid_month],
            vec![PaymentBreak::make_break(
                date!(2024 - 04 - 01),
                date!(2024 - 04 - 30),
            )],
        );
        let months = vec![paid_month, break_month];
        assert_eq!(
            validate_allocations(&months, &ctx),
            Err(PaymentValidationError::MonthAlreadyPaid(paid_month))
        );
    }
}
