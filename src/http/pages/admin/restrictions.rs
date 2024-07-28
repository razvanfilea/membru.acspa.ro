use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::{error, info};

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::restriction::Restriction;
use crate::model::user::UserUi;
use crate::utils::{date_formats, get_hour_structure_for_day};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(restrictions_page))
        .route("/", put(create_restriction))
        .route("/select_hour", post(select_hour))
        .route("/:date", delete(delete_restriction))
}

async fn get_restrictions(pool: &SqlitePool) -> Vec<Restriction> {
    query_as!(
        Restriction,
        "select * from reservations_restrictions order by date, hour"
    )
    .fetch_all(pool)
    .await
    .expect("Database error")
}

async fn restrictions_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/restrictions.html")]
    struct RestrictionsTemplate {
        user: UserUi,
        current_date: Date,
        restrictions: Vec<Restriction>,
    }

    RestrictionsTemplate {
        user: auth_session.user.unwrap(),
        restrictions: get_restrictions(&state.pool).await,
        current_date: OffsetDateTime::now_utc().date()
    }
}

#[derive(Deserialize)]
struct SelectDateForm {
    date: String,
    all_day: Option<String>,
}

async fn select_hour(
    State(state): State<AppState>,
    Form(form): Form<SelectDateForm>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/select_hour.html")]
    struct SelectHourTemplate {
        hours: Vec<u8>,
    }

    if form.all_day == Some("on".to_string()) {
        return ().into_response();
    }

    let date = Date::parse(&form.date, date_formats::ISO_DATE).unwrap();

    let hour_structure = get_hour_structure_for_day(&state, &date).await;

    SelectHourTemplate {
        hours: hour_structure.iter().collect(),
    }
    .into_response()
}

#[derive(Deserialize)]
struct NewRestriction {
    date: String,
    hour: Option<u8>,
    message: String,
}

async fn create_restriction(
    State(state): State<AppState>,
    Form(restriction): Form<NewRestriction>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/restrictions_content.html")]
    struct RestrictionsListTemplate {
        restrictions: Vec<Restriction>,
    }

    let date = Date::parse(&restriction.date, date_formats::ISO_DATE).unwrap();
    if let Some(hour) = restriction.hour {
        let hour_structure = get_hour_structure_for_day(&state, &date).await;
        if !hour_structure.is_hour_valid(hour) {
            error!("Invalid hour: {hour} for date: {}", restriction.date);

            return RestrictionsListTemplate {
                restrictions: get_restrictions(&state.pool).await,
            };
        }
    }

    let message = restriction.message.trim();

    query!(
        "insert into reservations_restrictions (date, hour, location, message) VALUES ($1, $2, $3, $4)",
        date,
        restriction.hour,
        state.location.id,
        message,
    )
    .execute(&state.pool)
    .await
    .expect("Database error");

    info!(
        "Add restriction with date: {date} hour: {} and message: {message}",
        restriction.hour.unwrap_or_default()
    );

    RestrictionsListTemplate {
        restrictions: get_restrictions(&state.pool).await,
    }
}

#[derive(Deserialize)]
struct HourQuery {
    hour: Option<u8>,
}

async fn delete_restriction(
    State(state): State<AppState>,
    Path(date): Path<String>,
    Query(query): Query<HourQuery>,
) -> impl IntoResponse {
    let date = Date::parse(&date, date_formats::ISO_DATE).unwrap();

    if let Some(hour) = query.hour {
        query!(
            "delete from reservations_restrictions where date = $1 and hour = $2",
            date,
            hour
        )
        .execute(&state.pool)
        .await
        .expect("Database error");
    } else {
        query!(
            "delete from reservations_restrictions where date = $1",
            date
        )
        .execute(&state.pool)
        .await
        .expect("Database error");
    }
}
