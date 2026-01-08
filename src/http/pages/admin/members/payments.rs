use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::model::payment::{PaymentAllocation, PaymentWithAllocations};
use crate::utils::local_date;
use crate::utils::queries::get_user;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{SqlitePool, query};
use time::{Date, Month};
use tracing::{info, warn};

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
                    month: a.month as u8,
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

#[derive(Deserialize, Debug)]
pub struct NewPayment {
    amount: f64,        // From input type="number" step="0.01"
    payment_date: Date, // From input type="date"
    months: String,     // From checkboxes (format: "M-YYYY")
    notes: Option<String>,
}

pub async fn add_payment(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    auth_session: AuthSession,
    Form(form): Form<NewPayment>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let member = get_user(&state.read_pool, user_id).await?;

    let current_year = local_date().year();
    let valid_year_range = member.member_since.year()..=current_year + 1;

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

    info!("Months: {}", form.months);
    let mut allocations_count = 0;
    for month_year in form.months.split(',') {
        let parts: Vec<&str> = month_year.split('-').collect();
        if parts.len() == 2 {
            let Some(month) = parts[0]
                .parse::<u8>()
                .ok()
                .and_then(|month| Month::try_from(month).ok())
                .map(|month| month as u8)
            else {
                warn!("Failed to parse invalid month: {}", parts[0]);
                continue;
            };
            let Some(year) = parts[1]
                .parse::<i32>()
                .ok()
                .filter(|year| valid_year_range.contains(year))
            else {
                warn!("Failed to parse invalid year: {}", parts[1]);
                continue;
            };

            query!(
                "insert into payment_allocations (payment_id, year, month) values ($1, $2, $3)",
                payment_id,
                year,
                month
            )
            .execute(tx.as_mut())
            .await?;
            allocations_count += 1;
        }
    }

    if allocations_count == 0 {
        return Err(HttpError::Message(
            "O plată trebuie să acopere cel puțin o lună".into(),
        ));
    }

    tx.commit().await?;

    Ok([("HX-Refresh", "true")].into_response())
}
