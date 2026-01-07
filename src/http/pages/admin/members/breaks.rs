use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::model::payment::PaymentBreak;
use crate::utils::date_formats;
use axum::Form;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{SqlitePool, query, query_as};
use time::Date;

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

pub async fn delete_break(State(state): State<AppState>, Path(break_id): Path<i64>) -> HttpResult {
    query!("delete from payment_breaks where id = $1", break_id)
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Refresh", "true")].into_response())
}
