use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail, bail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::payments_summary::{ViewYear, payments_changed_response};
use crate::model::payment_context::PaymentContext;
use crate::model::user::UserDetails;
use crate::utils::date_formats;
use crate::utils::dates::YearMonth;
use crate::utils::local_date;
use axum::Form;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use sqlx::{query, query_scalar};
use time::Date;
use tracing::info;

fn is_before_membership(date: Date, member: &UserDetails) -> bool {
    let member_join_month = YearMonth::from(member.member_since).to_date();
    date < member_join_month
}

#[derive(Deserialize, Debug)]
pub struct NewBreak {
    start_month: String,       // Format "2024-06"
    end_month: Option<String>, // Format "2024-08"
    reason: Option<String>,
    view_year: Option<i32>,
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

    let member = UserDetails::fetch(&state.read_pool, member_id).await?;
    let start_date = parse_month_input(&form.start_month).or_bail("Început de lună invalid")?;
    let end_date = parse_month_input(&form.end_month.unwrap_or(form.start_month))
        .or_bail("Sfârșit de lună invalid")?;

    if end_date < start_date {
        return Err(bail("Data selectată este invalidă"));
    }

    if is_before_membership(start_date, &member) {
        return Err(bail("Nu poți adăuga o pauză înainte de înscriere"));
    }

    let mut tx = state.write_pool.begin().await?;

    let ctx = PaymentContext::fetch(tx.as_mut(), member_id).await?;

    if ctx.overlaps_existing_break(start_date, end_date) {
        return Err(bail("Perioada se suprapune cu o pauză existentă"));
    }

    if ctx.overlaps_existing_payment(start_date, end_date) {
        return Err(bail("Perioada se suprapune cu o lună deja plătită"));
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

    let year = form.view_year.unwrap_or_else(|| local_date().year());
    payments_changed_response(&state.read_pool, member_id, year, "Pauza a fost adăugată").await
}

pub async fn delete_break(
    State(state): State<AppState>,
    Path(break_id): Path<i64>,
    Query(view): Query<ViewYear>,
) -> HttpResult {
    let member_id = query_scalar!(
        "delete from payment_breaks where id = $1 returning user_id",
        break_id
    )
    .fetch_optional(&state.write_pool)
    .await?
    .or_bail("Pauza nu există")?;

    payments_changed_response(&state.read_pool, member_id, view.year(), "Pauza a fost ștearsă").await
}
