use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::{date_formats, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::routing::{delete, get, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::info;

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
    created_at: OffsetDateTime,
}

async fn tournament_days(pool: &SqlitePool) -> Vec<Tournament> {
    query!("select date, description, slots_start_hour, slot_duration, created_at from alternative_days where type = 'turneu' order by date desc, created_at")
        .fetch_all(pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|record| Tournament {
            date: record.date,
            description: record.description.unwrap_or_default(),
            start_hour: record.slots_start_hour,
            duration: record.slot_duration,
            created_at: record.created_at,
        })
        .collect()
}

async fn tournaments_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
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
    start_hour: i64,
    duration: i64,
}

async fn create_tournament(
    State(state): State<AppState>,
    Form(tournament): Form<NewTournament>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/tournaments_content.html")]
    struct TournamentsListTemplate {
        tournaments: Vec<Tournament>,
    }

    let date = Date::parse(&tournament.date, date_formats::ISO_DATE).ok();
    let description = tournament
        .description
        .map(|date| date.trim().to_string())
        .filter(|date| !date.is_empty());

    if let Some(date) = date {
        query!(
            "insert into alternative_days (date, description, type, slots_start_hour, slot_duration, slots_per_day) VALUES ($1, $2, 'turneu', $3, $4, 1)",
            date,
            description,
            tournament.start_hour,
            tournament.duration
        )
            .execute(&state.write_pool)
            .await
            .expect("Database error");

        info!(
            "Add tournament with date: {date} and description {}",
            description.unwrap_or_default()
        );
    }

    TournamentsListTemplate {
        tournaments: tournament_days(&state.read_pool).await,
    }
}

async fn delete_tournament(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> impl IntoResponse {
    let date = Date::parse(&date, date_formats::ISO_DATE).unwrap();

    query!("delete from alternative_days where date = $1", date)
        .execute(&state.write_pool)
        .await
        .expect("Database error");
}
