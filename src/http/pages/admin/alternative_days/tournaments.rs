use crate::http::error::HttpResult;
use crate::http::pages::admin::alternative_days::{
    add_alternative_day, alternative_days, delete_alternative_day, AlternativeDay,
    NewAlternativeDay,
};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{delete, get, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{Error, SqlitePool};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(tournaments_page))
        .route("/", put(create_tournament))
        .route("/{date}", delete(delete_tournament))
}

async fn tournament_days(pool: &SqlitePool) -> Result<Vec<AlternativeDay>, Error> {
    alternative_days(pool, "turneu").await
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
        tournaments: Vec<AlternativeDay>,
    }

    TournamentsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        tournaments: tournament_days(&state.read_pool).await?,
        current_date: local_time().date(),
    }
    .try_into_response()
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
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "components/admin/tournaments_content.html")]
    struct TournamentsListTemplate {
        tournaments: Vec<AlternativeDay>,
    }

    let capacity = tournament
        .capacity
        .and_then(|capacity| capacity.parse::<u8>().ok());

    let day = NewAlternativeDay {
        date: tournament.date.clone(),
        description: tournament.description.clone(),
        start_hour: tournament.start_hour,
        duration: tournament.duration,
        slots_per_day: 1,
        capacity,
    };

    if let Some(error_response) = add_alternative_day(state.write_pool, day, "turneu").await? {
        return Ok(error_response);
    }

    info!(
        "Added tournament with date: {} and description {}",
        tournament.date,
        tournament.description.unwrap_or_default()
    );

    TournamentsListTemplate {
        tournaments: tournament_days(&state.read_pool).await?,
    }
    .try_into_response()
}

pub async fn delete_tournament(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> HttpResult {
    delete_alternative_day(state, date).await
}
