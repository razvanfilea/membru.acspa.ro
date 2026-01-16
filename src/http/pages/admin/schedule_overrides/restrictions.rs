use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::admin::schedule_overrides::calendar::day_details_response;
use crate::model::day_structure::DayStructure;
use crate::model::restriction::Restriction;
use crate::model::user_reservation::UserReservation;
use crate::utils::date_formats;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::{delete, put};
use axum_extra::extract::Form as AxumExtraForm;
use serde::Deserialize;
use sqlx::{Error, SqlitePool, query, query_as};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", put(create_restriction))
        .route("/", delete(delete_restriction))
}

pub async fn get_restrictions_for_day(
    pool: &SqlitePool,
    date: Date,
) -> Result<Vec<Restriction>, Error> {
    query_as!(
        Restriction,
        "select date, hour, message, created_at from restrictions where date = $1 order by hour",
        date
    )
    .fetch_all(pool)
    .await
}

pub async fn get_restrictions_for_month(
    pool: &SqlitePool,
    month_year: Date,
) -> Result<Vec<Restriction>, Error> {
    query_as!(
        Restriction,
        "select date, hour, message, created_at from restrictions
         where strftime('%m%Y', date) = strftime('%m%Y', $1)
         order by hour",
        month_year
    )
    .fetch_all(pool)
    .await
}

#[derive(Deserialize)]
struct NewRestriction {
    date: String,
    hour: Option<Vec<u8>>,
    message: String,
}

async fn create_restriction(
    State(state): State<AppState>,
    AxumExtraForm(restriction): AxumExtraForm<NewRestriction>,
) -> HttpResult {
    let message = restriction.message.trim();
    let date = Date::parse(&restriction.date, date_formats::ISO_DATE).unwrap();
    let day_structure =
        DayStructure::fetch_or_default(&state.read_pool, date, &state.location).await?;
    let mut tx = state.write_pool.begin().await?;

    if let Some(hours) = restriction.hour {
        for hour in hours {
            if !day_structure.is_hour_valid(hour) {
                continue;
            }

            UserReservation::delete_on_day(tx.as_mut(), date, Some(hour)).await?;

            query!(
                "insert or replace into restrictions (date, hour, location, message) values ($1, $2, $3, $4)",
                date,
                hour,
                state.location.id,
                message,
            )
                .execute(tx.as_mut())
                .await?;
        }
    } else {
        UserReservation::delete_on_day(tx.as_mut(), date, None).await?;

        query!(
            "insert into restrictions (date, location, message) values ($1, $2, $3)",
            date,
            state.location.id,
            message,
        )
        .execute(tx.as_mut())
        .await?;

        info!("Add restriction with date: {date}, for the entire day and message: {message}");
    };

    tx.commit().await?;

    day_details_response(state, date).await
}

#[derive(Deserialize)]
struct HourQuery {
    date: Date,
    hour: Option<u8>,
}

async fn delete_restriction(
    State(state): State<AppState>,
    Query(query): Query<HourQuery>,
) -> HttpResult {
    let date = query.date;
    if let Some(hour) = query.hour {
        query!(
            "delete from restrictions where date = $1 and hour = $2",
            date,
            hour
        )
        .execute(&state.write_pool)
        .await?;
    } else {
        query!("delete from restrictions where date = $1", date)
            .execute(&state.write_pool)
            .await?;
    }

    day_details_response(state, date).await
}
