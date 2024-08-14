use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::free_day::FreeDay;
use crate::model::user::UserUi;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::routing::{delete, get, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, SqlitePool};
use time::Date;
use time::macros::format_description;
use tracing::info;
use crate::utils::{date_formats, local_time};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(free_days_page))
        .route("/", put(create_free_day))
        .route("/:date", delete(delete_free_day))
}

async fn get_free_days(pool: &SqlitePool) -> Vec<FreeDay> {
    query_as!(FreeDay, "select * from free_days order by date")
        .fetch_all(pool)
        .await
        .expect("Database error")
}

async fn free_days_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/free_days.html")]
    struct FreeDaysTemplate {
        user: UserUi,
        current_date: Date,
        free_days: Vec<FreeDay>,
    }

    FreeDaysTemplate {
        user: auth_session.user.unwrap(),
        free_days: get_free_days(&state.pool).await,
        current_date: local_time().date(),
    }
}

#[derive(Deserialize)]
struct NewFreeDay {
    date: String,
    description: Option<String>,
}

async fn create_free_day(
    State(state): State<AppState>,
    Form(day): Form<NewFreeDay>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/free_days_content.html")]
    struct FreeDaysListTemplate {
        free_days: Vec<FreeDay>,
    }

    let date = Date::parse(&day.date, date_formats::ISO_DATE).ok();
    let description = day
        .description
        .map(|date| date.trim().to_string())
        .filter(|date| !date.is_empty());

    if let Some(date) = date {
        query!(
            "insert into free_days (date, description) VALUES ($1, $2)",
            date,
            description,
        )
        .execute(&state.pool)
        .await
        .expect("Database error");

        info!(
            "Add free day with date: {date} and description {}",
            description.unwrap_or_default()
        );
    }

    FreeDaysListTemplate {
        free_days: get_free_days(&state.pool).await,
    }
}

async fn delete_free_day(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> impl IntoResponse {
    let date = Date::parse(&date, format_description!("[year]-[month]-[day]")).unwrap();

    query!("delete from free_days where date = $1", date)
        .execute(&state.pool)
        .await
        .expect("Database error");
}
