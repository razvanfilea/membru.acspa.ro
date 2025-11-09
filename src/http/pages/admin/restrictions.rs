use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::restriction::Restriction;
use crate::model::user::User;
use crate::utils::queries::get_day_structure;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{SqlitePool, query, query_as};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(restrictions_page))
        .route("/", put(create_restriction))
        .route("/select_hour", post(select_hour))
        .route("/{date}", delete(delete_restriction))
}

async fn get_restrictions(pool: &SqlitePool) -> sqlx::Result<Vec<Restriction>> {
    query_as!(
        Restriction,
        "select date, hour, message, created_at from restrictions order by date desc, hour"
    )
    .fetch_all(pool)
    .await
}

async fn restrictions_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/restrictions/restrictions_page.html")]
    struct RestrictionsTemplate {
        user: User,
        current_date: Date,
        restrictions: Vec<Restriction>,
        hours: Vec<u8>,
    }

    let current_date = local_time().date();
    let day_structure = get_day_structure(&state, current_date).await;

    RestrictionsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        restrictions: get_restrictions(&state.read_pool).await?,
        current_date,
        hours: day_structure.iter().collect(),
    }
    .try_into_response()
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
    #[template(path = "admin/restrictions/select_hour.html")]
    struct SelectHourTemplate {
        hours: Vec<u8>,
    }

    if form.all_day == Some("on".to_string()) {
        return ().into_response();
    }

    let date = Date::parse(&form.date, date_formats::ISO_DATE).unwrap();

    let day_structure = get_day_structure(&state, date).await;

    SelectHourTemplate {
        hours: day_structure.iter().collect(),
    }
    .into_response()
}

#[derive(Deserialize)]
struct NewRestriction {
    date: String,
    hour: Option<Vec<u8>>,
    message: String,
}

use axum_extra::extract::Form as AxumExtraForm;

async fn create_restriction(
    State(state): State<AppState>,
    AxumExtraForm(restriction): AxumExtraForm<NewRestriction>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/restrictions/restrictions_page.html", block = "list")]
    struct RestrictionsListTemplate {
        restrictions: Vec<Restriction>,
    }

    let message = restriction.message.trim();
    let date = Date::parse(&restriction.date, date_formats::ISO_DATE).unwrap();
    let day_structure = get_day_structure(&state, date).await;

    let Some(hours) = restriction.hour else {
        query!(
            "insert into restrictions (date, location, message) VALUES ($1, $2, $3)",
            date,
            state.location.id,
            message,
        )
        .execute(&state.write_pool)
        .await?;

        info!("Add restriction with date: {date}, for the entire day and message: {message}");

        return RestrictionsListTemplate {
            restrictions: get_restrictions(&state.read_pool).await?,
        }
        .try_into_response();
    };

    for hour in hours {
        if !day_structure.is_hour_valid(hour) {
            continue;
        }

        query!(
            "insert or replace into restrictions (date, hour, location, message) VALUES ($1, $2, $3, $4)",
            date,
            hour,
            state.location.id,
            message,
        )
        .execute(&state.write_pool)
        .await?;
    }

    RestrictionsListTemplate {
        restrictions: get_restrictions(&state.read_pool).await?,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct HourQuery {
    hour: Option<u8>,
}

async fn delete_restriction(
    State(state): State<AppState>,
    Path(date): Path<String>,
    Query(query): Query<HourQuery>,
) -> HttpResult {
    let date = Date::parse(&date, date_formats::ISO_DATE).unwrap();

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

    Ok(().into_response())
}
