use crate::http::pages::admin::tournaments::delete_tournament;
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::day_structure::HOLIDAY_DAY_STRUCTURE;
use crate::model::user::User;
use crate::utils::queries::alt_day_exists;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, State};
use axum::routing::{delete, get, put};
use axum::{Form, Router};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::{query, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::info;
use template_response::TemplateResponse;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(free_days_page))
        .route("/", put(create_free_day))
        .route("/:date", delete(delete_free_day))
}

struct FreeDay {
    date: Date,
    description: String,
    created_at: OffsetDateTime,
}

async fn get_free_days(pool: &SqlitePool) -> Vec<FreeDay> {
    query!("select date, description, created_at from alternative_days where type = 'holiday' order by date desc, created_at")
        .fetch_all(pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|day| FreeDay {
            date: day.date,
            description: day.description.unwrap_or_default(),
            created_at: day.created_at,
        })
        .collect()
}

async fn free_days_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template, TemplateResponse)]
    #[template(path = "pages/admin/free_days.html")]
    struct FreeDaysTemplate {
        user: User,
        current_date: Date,
        free_days: Vec<FreeDay>,
    }

    FreeDaysTemplate {
        user: auth_session.user.expect("User should be logged in"),
        free_days: get_free_days(&state.read_pool).await,
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
    #[derive(Template, TemplateResponse)]
    #[template(path = "components/admin/free_days_content.html")]
    struct FreeDaysListTemplate {
        free_days: Vec<FreeDay>,
    }

    let Some(date) = Date::parse(&day.date, date_formats::ISO_DATE).ok() else {
        return error_bubble_response("Data selectata nu este valida");
    };
    let description = day
        .description
        .map(|date| date.trim().to_string())
        .filter(|date| !date.is_empty());

    let day_structure = &HOLIDAY_DAY_STRUCTURE;
    if alt_day_exists(&state.read_pool, date).await {
        return error_bubble_response(format!(
            "Deja exists o zi libera/turneu pe data de {}",
            date.format(date_formats::READABLE_DATE).unwrap()
        ));
    }

    query!(
        "insert into alternative_days (date, description, type, slots_start_hour, slot_duration, slots_per_day) VALUES ($1, $2, 'holiday', $3, $4, $5)",
        date,
        description,
        day_structure.slots_start_hour,
        day_structure.slot_duration,
        day_structure.slots_per_day
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    info!(
        "Add free day with date: {date} and description {}",
        description.unwrap_or_default()
    );

    FreeDaysListTemplate {
        free_days: get_free_days(&state.read_pool).await,
    }
    .into_response()
}

async fn delete_free_day(state: State<AppState>, date: Path<String>) {
    delete_tournament(state, date).await
}
