use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail, OrBailWith};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::payments_summary::{
    ViewYear, payments_changed_response, view_year_or_current,
};
use crate::model::payment_context::PaymentContext;
use crate::model::user::UserDetails;
use crate::utils::dates::YearMonth;
use crate::utils::local_date;
use axum::Form;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use sqlx::{query, query_scalar};
use std::fmt;
use time::{Date, Month};
use tracing::info;

#[derive(Deserialize, Debug)]
pub(super) struct NewPayment {
    amount: f64,
    payment_date: Date,
    months: String,     // Format "M-YYYY"
    notes: Option<String>,
    view_year: Option<i32>,
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

pub(super) async fn add_payment(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewPayment>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let member = UserDetails::fetch(&state.read_pool, member_id).await?;

    let amount_cents = validate_and_convert_amount(form.amount).or_bail_with(|e| e)?;

    let current_year = local_date().year();
    let requested_allocations =
        parse_month_allocations(&form.months, member.member_since, current_year)
            .or_bail_with(|e| e)?;

    let mut tx = state.write_pool.begin().await?;

    let ctx = PaymentContext::fetch(tx.as_mut(), member_id).await?;

    validate_allocations(&requested_allocations, &ctx).or_bail_with(|e| e)?;

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

    let year = view_year_or_current(form.view_year);
    payments_changed_response(
        &state.read_pool,
        member_id,
        year,
        "Plata a fost înregistrată",
    )
    .await
}

pub(super) async fn delete_payment(
    State(state): State<AppState>,
    Path(payment_id): Path<i64>,
    Query(view): Query<ViewYear>,
) -> HttpResult {
    let member_id = query_scalar!(
        "delete from payments where id = $1 returning user_id",
        payment_id
    )
    .fetch_optional(&state.write_pool)
    .await?
    .or_bail("Plata nu există")?;

    payments_changed_response(
        &state.read_pool,
        member_id,
        view.year(),
        "Plata a fost ștearsă",
    )
    .await
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
        assert_eq!(
            parse_month_allocations("3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        assert_eq!(
            parse_month_allocations("1-2024,2-2024,3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![
                YearMonth::new(2024, Month::January),
                YearMonth::new(2024, Month::February),
                YearMonth::new(2024, Month::March),
            ])
        );

        assert_eq!(
            parse_month_allocations("invalid,3-2024", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        assert_eq!(
            parse_month_allocations("1-2020,6-2020", date!(2020 - 03 - 15), 2024),
            Ok(vec![YearMonth::new(2020, Month::June)])
        );

        // Year outside valid range is skipped (member_since 2020, current_year 2024, range 2020-2025)
        assert_eq!(
            parse_month_allocations("1-2019,3-2024,1-2027", date!(2020 - 01 - 01), 2024),
            Ok(vec![YearMonth::new(2024, Month::March)])
        );

        assert_eq!(
            parse_month_allocations("", date!(2020 - 01 - 01), 2024),
            Err(PaymentValidationError::NoValidMonths)
        );

        assert_eq!(
            parse_month_allocations("invalid,bad,nope", date!(2020 - 01 - 01), 2024),
            Err(PaymentValidationError::NoValidMonths)
        );
    }

    #[test]
    fn test_validate_allocations() {
        let ctx = PaymentContext::new(vec![], vec![]);
        let months = vec![
            YearMonth::new(2024, Month::January),
            YearMonth::new(2024, Month::February),
        ];
        assert_eq!(validate_allocations(&months, &ctx), Ok(()));

        let paid_month = YearMonth::new(2024, Month::January);
        let ctx = PaymentContext::new(vec![paid_month], vec![]);
        let months = vec![paid_month, YearMonth::new(2024, Month::February)];
        assert_eq!(
            validate_allocations(&months, &ctx),
            Err(PaymentValidationError::MonthAlreadyPaid(paid_month))
        );

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
