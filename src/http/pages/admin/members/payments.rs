use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::breaks::get_user_payment_breaks;
use crate::model::payment::PaymentWithAllocations;
use crate::model::user::User;
use crate::utils::dates::YearMonth;
use crate::utils::local_date;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{SqliteExecutor, SqlitePool, query};
use time::{Date, Month};
use tracing::info;

pub async fn get_user_payments(
    pool: &SqlitePool,
    user_id: i64,
) -> sqlx::Result<Vec<PaymentWithAllocations>> {
    let payments = query!(
        "select p.id, amount, payment_date, notes, created_at, created_by, u.name as created_by_name from payments p
         join users u on u.id = p.created_by
         where user_id = $1 order by payment_date desc",
        user_id
    )
        .fetch_all(pool)
        .await?;

    let all_allocations = query!(
        "select payment_id, year, month from payment_allocations where payment_id in (select id from payments where user_id = ?) order by year desc, month desc",
        user_id
    )
        .fetch_all(pool)
        .await?;

    Ok(payments
        .into_iter()
        .map(|p| {
            let allocations = all_allocations
                .iter()
                .filter(|a| a.payment_id == p.id)
                .filter_map(|a| {
                    Some(YearMonth::new(
                        a.year as i32,
                        Month::try_from(a.month as u8).ok()?,
                    ))
                })
                .collect();

            PaymentWithAllocations {
                id: p.id,
                amount: p.amount,
                payment_date: p.payment_date,
                notes: p.notes.filter(|notes| !notes.is_empty()),
                created_at: p.created_at,
                created_by: p.created_by,
                created_by_name: p.created_by_name,
                allocations,
            }
        })
        .collect())
}

pub async fn get_payment_allocations(
    executor: impl SqliteExecutor<'_>,
    user_id: i64,
) -> sqlx::Result<Vec<YearMonth>> {
    query!(
        "select year, month from payment_allocations where payment_id in (select id from payments where user_id = ?)",
        user_id
    )
        .fetch_all(executor)
        .await
        .map(|vec| vec
            .into_iter()
            .filter_map(|record| Some(YearMonth::new(record.year as i32, Month::try_from(record.month as u8).ok()?)))
            .collect()
        )
}

#[derive(Deserialize, Debug)]
pub struct NewPayment {
    amount: f64,        // From input type="number" step="0.01"
    payment_date: Date, // From input type="date"
    months: String,     // From checkboxes (format: "M-YYYY")
    notes: Option<String>,
}

pub async fn add_payment(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewPayment>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let member = User::fetch(&state.read_pool, member_id).await?;

    if form.amount <= 0.0 {
        return Err(HttpError::Message("Suma trebuie să fie pozitivă".into()));
    }

    let current_year = local_date().year();
    let valid_year_range = member.member_since.year()..=current_year + 1;

    // Parse requested months first
    let requested_allocations: Vec<_> = form
        .months
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

            let joining_month = YearMonth::from(member.member_since);

            // Skip months before they joined
            Some(YearMonth::new(year, month)).filter(|parsed| parsed >= &joining_month)
        })
        .collect();

    if requested_allocations.is_empty() {
        return Err(HttpError::Message(
            "O plată trebuie să acopere cel puțin o lună validă".into(),
        ));
    }

    let mut tx = state.write_pool.begin().await?;

    let existing_allocations = get_payment_allocations(tx.as_mut(), member_id).await?;
    let existing_breaks = get_user_payment_breaks(tx.as_mut(), member_id).await?;

    for requested in &requested_allocations {
        if existing_allocations.contains(requested) {
            return Err(HttpError::Message(format!(
                "Luna {}-{} este deja plătită",
                requested.month, requested.year
            )));
        }

        // Check for breaks
        let req_date = Date::from_calendar_date(requested.year, requested.month, 1).unwrap();

        let is_break = existing_breaks
            .iter()
            .any(|b| (b.start_date..=b.end_date).contains(&req_date));

        if is_break {
            return Err(HttpError::Message(format!(
                "Luna {}-{} este marcată ca pauză",
                requested.month as u8, requested.year
            )));
        }
    }

    // Convert amount to cents (integer) for storage
    let amount_cents = (form.amount * 100.0).round() as i64;

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
