use crate::http::pages::notification_template::error_bubble_response;
use crate::http::pages::AuthSession;
use crate::http::AppState;
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
        .route("/", get(tournaments_page))
        .route("/", put(create_tournament))
        .route("/:date", delete(delete_tournament))
}

struct Tournament {
    date: Date,
    description: String,
    start_hour: i64,
    duration: i64,
    slot_capacity: Option<i64>,
    created_at: OffsetDateTime,
}

async fn tournament_days(pool: &SqlitePool) -> Vec<Tournament> {
    query!("select date, description, slots_start_hour, slot_duration, slot_capacity, created_at from alternative_days where type = 'turneu' order by date desc, created_at")
        .fetch_all(pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|record| Tournament {
            date: record.date,
            description: record.description.unwrap_or_default(),
            start_hour: record.slots_start_hour,
            duration: record.slot_duration,
            slot_capacity: record.slot_capacity,
            created_at: record.created_at,
        })
        .collect()
}

async fn tournaments_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template, TemplateResponse)]
    #[template(path = "pages/admin/tournaments.html")]
    struct TournamentsTemplate {
        user: User,
        current_date: Date,
        tournaments: Vec<Tournament>,
    }

    TournamentsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        tournaments: tournament_days(&state.read_pool).await,
        current_date: local_time().date(),
    }
}

#[derive(Deserialize)]
struct NewTournament {
    date: String,
    description: Option<String>,
    start_hour: u8,
    duration: u8,
    capacity: Option<String>,
}

async fn create_tournament(
    State(state): State<AppState>,
    Form(tournament): Form<NewTournament>,
) -> impl IntoResponse {
    #[derive(Template, TemplateResponse)]
    #[template(path = "components/admin/tournaments_content.html")]
    struct TournamentsListTemplate {
        tournaments: Vec<Tournament>,
    }

    let Some(date) = Date::parse(&tournament.date, date_formats::ISO_DATE).ok() else {
        return error_bubble_response("Data selectată nu este valida");
    };
    let description = tournament
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());

    if alt_day_exists(&state.read_pool, date).await {
        return error_bubble_response(format!(
            "Deja exists o zi libera/turneu pe data de {}",
            date.format(date_formats::READABLE_DATE).unwrap()
        ));
    }

    let capacity = tournament
        .capacity
        .and_then(|capacity| capacity.parse::<u8>().ok());

    query!(
        "insert into alternative_days (date, description, type, slots_start_hour, slot_duration, slot_capacity, slots_per_day) VALUES ($1, $2, 'turneu', $3, $4, $5, 1)",
        date,
        description,
        tournament.start_hour,
        tournament.duration,
        capacity
    )
        .execute(&state.write_pool)
        .await
        .expect("Database error");

    info!(
        "Add tournament with date: {date} and description {}",
        description.unwrap_or_default()
    );

    TournamentsListTemplate {
        tournaments: tournament_days(&state.read_pool).await,
    }
    .into_response()
}

pub async fn delete_tournament(State(state): State<AppState>, Path(date): Path<String>) {
    let date = Date::parse(&date, date_formats::ISO_DATE).unwrap();

    let mut tx = state
        .write_pool
        .begin()
        .await
        .expect("Failed to create transaction");

    query!("delete from reservations where date = $1", date)
        .execute(tx.as_mut())
        .await
        .expect("Database error");

    query!("delete from alternative_days where date = $1", date)
        .execute(tx.as_mut())
        .await
        .expect("Database error");

    tx.commit().await.expect("Failed to delete alternative day");
}
