use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::payments::get_payment_allocations;
use crate::model::payment::PaymentBreak;
use crate::utils::date_formats;
use crate::utils::queries::get_user;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{SqliteExecutor, query, query_as};
use time::Date;
use tracing::info;

pub async fn get_user_payment_breaks(
    executor: impl SqliteExecutor<'_>,
    user_id: i64,
) -> sqlx::Result<Vec<PaymentBreak>> {
    query_as!(
        PaymentBreak,
        "select m.*, u.name as created_by_name
         from payment_breaks m join users u on u.id = m.created_by
         where user_id = $1 order by start_date desc",
        user_id
    )
    .fetch_all(executor)
    .await
}

#[derive(Deserialize, Debug)]
pub struct NewBreak {
    start_month: String, // Format "2024-06"
    end_month: String,   // Format "2024-08"
    reason: Option<String>,
}

pub async fn add_break(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewBreak>,
) -> HttpResult {
    fn parse_month_input(input: &str) -> Option<Date> {
        let date_str = format!("{}-01", input);
        Date::parse(&date_str, date_formats::ISO_DATE).ok()
    }

    let created_by = auth_session.user.ok_or(HttpError::Unauthorized)?;

    let member = get_user(&state.read_pool, member_id).await?;
    let start_date = parse_month_input(&form.start_month).or_bail("Început de lună invalid")?;
    let end_date = parse_month_input(&form.end_month).or_bail("Sfârșit de lună invalid")?;

    if end_date < start_date {
        return Err(HttpError::Message("Data selectată este invalidă".into()));
    }

    // Validate that break is not before membership
    let member_join_month =
        Date::from_calendar_date(member.member_since.year(), member.member_since.month(), 1)
            .unwrap_or(member.member_since);

    if start_date < member_join_month {
        return Err(HttpError::Message(
            "Nu poți adăuga o pauză înainte de înscriere".into(),
        ));
    }

    let mut tx = state.write_pool.begin().await?;

    let existing_breaks = get_user_payment_breaks(tx.as_mut(), member_id).await?;
    let existing_allocations = get_payment_allocations(tx.as_mut(), member_id).await?;

    for brk in existing_breaks {
        if start_date <= brk.end_date && brk.start_date <= end_date {
            return Err(HttpError::Message(
                "Perioada se suprapune cu o pauză existentă".into(),
            ));
        }
    }

    for payment in existing_allocations {
        let pay_date = Date::from_calendar_date(payment.year, payment.month, 1).unwrap();

        if pay_date >= start_date && pay_date <= end_date {
            return Err(HttpError::Message(
                "Perioada se suprapune cu o lună deja plătită".into(),
            ));
        }
    }

    let reason = form.reason.filter(|reason| !reason.is_empty());
    query!(
        "insert into payment_breaks (user_id, start_date, end_date, reason, created_by)
         values ($1, $2, $3, $4, $5)",
        member_id,
        start_date,
        end_date,
        reason,
        created_by.id
    )
    .execute(tx.as_mut())
    .await?;

    info!(
        "Payment Break added for member {}: {} to {}",
        member_id, start_date, end_date
    );

    tx.commit().await?;

    Ok([("HX-Refresh", "true")].into_response())
}

pub async fn delete_break(State(state): State<AppState>, Path(break_id): Path<i64>) -> HttpResult {
    query!("delete from payment_breaks where id = $1", break_id)
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Refresh", "true")].into_response())
}
