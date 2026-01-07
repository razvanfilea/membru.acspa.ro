use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::model::payment::{PaymentAllocation, PaymentBreak, PaymentWithAllocations};
use crate::utils::date_formats;
use axum::Form;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{SqlitePool, query, query_as};
use time::Date;

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
                .map(|a| PaymentAllocation {
                    year: a.year as i32,
                    month: a.month as i8,
                })
                .collect();

            PaymentWithAllocations {
                amount: p.amount,
                payment_date: p.payment_date,
                notes: p.notes,
                created_at: p.created_at,
                created_by: p.created_by,
                created_by_name: p.created_by_name,
                allocations,
            }
        })
        .collect())
}

pub async fn get_user_payment_breaks(
    pool: &SqlitePool,
    user_id: i64,
) -> sqlx::Result<Vec<PaymentBreak>> {
    query_as!(
        PaymentBreak,
        "select m.*, u.name as created_by_name
         from payment_breaks m join users u on u.id = m.created_by
         where user_id = $1 order by start_date desc",
        user_id
    )
    .fetch_all(pool)
    .await
}

#[derive(Deserialize, Debug)]
pub struct NewPayment {
    amount: f64,         // From input type="number" step="0.01"
    payment_date: Date,  // From input type="date"
    months: Vec<String>, // From checkboxes (format: "M-YYYY")
    notes: Option<String>,
}

pub async fn add_payment(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    auth_session: AuthSession,
    axum_extra::extract::Form(form): axum_extra::extract::Form<NewPayment>,
) -> HttpResult {
    let user = auth_session.user.expect("User should be logged in");
    let mut tx = state.write_pool.begin().await?;

    // Convert amount to cents (integer) for storage
    let amount_cents = (form.amount * 100.0).round() as i64;

    // Insert the main payment record
    let payment_id = query!(
        "insert into payments (user_id, amount, payment_date, notes, created_by)
         values ($1, $2, $3, $4, $5) returning id",
        user_id,
        amount_cents,
        form.payment_date,
        form.notes,
        user.id,
    )
    .fetch_one(tx.as_mut())
    .await?
    .id;

    for month_year in form.months {
        // Parse "6-2024" format
        let parts: Vec<&str> = month_year.split('-').collect();
        if parts.len() == 2 {
            let month = parts[0].parse::<i8>().unwrap_or(0);
            let year = parts[1].parse::<i32>().unwrap_or(0);

            query!(
                "insert into payment_allocations (payment_id, year, month) values ($1, $2, $3)",
                payment_id,
                year,
                month
            )
            .execute(tx.as_mut())
            .await?;
        }
    }

    tx.commit().await?;

    Ok([("HX-Refresh", "true")].into_response())
}

#[derive(Deserialize, Debug)]
pub struct NewBreak {
    start_month: String, // Format "2024-06"
    end_month: String,   // Format "2024-08"
    reason: Option<String>,
}

pub async fn add_break(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewBreak>,
) -> HttpResult {
    fn parse_month_input(input: &str) -> Option<Date> {
        let date_str = format!("{}-01", input);
        Date::parse(&date_str, date_formats::ISO_DATE).ok()
    }

    let created_by = auth_session.user.expect("User should be logged in");
    let start_date = parse_month_input(&form.start_month).expect("Invalid start month");
    let end_date = parse_month_input(&form.end_month).expect("Invalid end month");

    if end_date < start_date {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    query!(
        "insert into payment_breaks (user_id, start_date, end_date, reason, created_by)
         values ($1, $2, $3, $4, $5)",
        user_id,
        start_date,
        end_date,
        form.reason,
        created_by.id
    )
    .execute(&state.write_pool)
    .await?;

    Ok([("HX-Refresh", "true")].into_response())
}
