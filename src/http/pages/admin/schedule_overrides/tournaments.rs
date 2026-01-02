use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::schedule_overrides::{
    AlternativeDay, AlternativeDayType, NewAlternativeDay, add_alternative_day,
    delete_alternative_day, get_alternative_day, get_alternative_days,
};
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::utils::date_formats::ISO_DATE;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{Error, Executor, Sqlite, SqlitePool, query};
use time::Date;
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(tournaments_page))
        .route("/new", get(new_tournament_page))
        .route("/new", post(create_tournament))
        .route("/edit/{date}", get(edit_tournament_page))
        .route("/edit/{date}", post(update_tournament))
        .route("/{date}", delete(delete_tournament))
}

pub async fn get_tournament_day(
    executor: impl Executor<'_, Database = Sqlite>,
    date: Date,
) -> Result<Option<AlternativeDay>, Error> {
    get_alternative_day(executor, AlternativeDayType::Tournament, date).await
}

pub async fn get_tournament_days(
    pool: &SqlitePool,
    year_month: Option<Date>,
) -> Result<Vec<AlternativeDay>, Error> {
    get_alternative_days(pool, AlternativeDayType::Tournament, year_month).await
}

async fn tournaments_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/tournaments/list_page.html")]
    struct TournamentsTemplate {
        user: User,
        upcoming: Vec<AlternativeDay>,
        past: Vec<AlternativeDay>,
    }

    let today = local_time().date();
    let (upcoming, past) = get_tournament_days(&state.read_pool, None)
        .await?
        .into_iter()
        .partition(|t| t.date >= today);

    TournamentsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        upcoming,
        past,
    }
    .try_into_response()
}

#[derive(Template)]
#[template(path = "admin/tournaments/new_edit_page.html")]
struct NewOrEditTournamentTemplate {
    user: User,
    current: Option<AlternativeDay>,
    current_date: Date,
}

async fn new_tournament_page(auth_session: AuthSession) -> HttpResult {
    NewOrEditTournamentTemplate {
        user: auth_session.user.expect("User should be logged in"),
        current: None,
        current_date: local_time().date(),
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct NewTournament {
    date: String,
    description: Option<String>,
    start_hour: u8,
    #[serde(default)]
    start_minute: u8,
    duration: u8,
    capacity: Option<String>,
    consumes_reservation: Option<String>,
}

async fn create_tournament(
    State(state): State<AppState>,
    Form(tournament): Form<NewTournament>,
) -> HttpResult {
    let Ok(date) = Date::parse(&tournament.date, date_formats::ISO_DATE) else {
        return Err(HttpError::Text("Data selectata nu este validă".to_string()));
    };

    let capacity = tournament
        .capacity
        .and_then(|capacity| capacity.parse::<u8>().ok());

    let day = NewAlternativeDay {
        date,
        description: tournament.description.clone(),
        start_hour: tournament.start_hour,
        start_minute: tournament.start_minute,
        duration: tournament.duration,
        slots_per_day: 1,
        capacity,
        consumes_reservation: tournament.consumes_reservation == Some("on".to_string()),
    };

    add_alternative_day(&state.write_pool, day, AlternativeDayType::Tournament).await?;

    info!(
        "Added tournament with date: {} and description {}",
        tournament.date,
        tournament.description.unwrap_or_default()
    );

    Ok([("HX-Redirect", "/admin/tournaments")].into_response())
}

async fn edit_tournament_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(date): Path<String>,
) -> HttpResult {
    let date = Date::parse(&date, ISO_DATE).expect("Data este invalida");
    let Some(current) = get_tournament_day(&state.read_pool, date).await? else {
        return Ok(error_bubble_response("Nu exista acest turneu"));
    };

    NewOrEditTournamentTemplate {
        user: auth_session.user.expect("User should be logged in"),
        current: Some(current),
        current_date: local_time().date(),
    }
    .try_into_response()
}

#[derive(Deserialize, Debug)]
struct UpdatedTournament {
    description: String,
    start_hour: i64,
    #[serde(default)]
    start_minute: u8,
    duration: u8,
    capacity: Option<String>,
    consumes_reservation: Option<String>,
}

async fn update_tournament(
    State(state): State<AppState>,
    Path(date): Path<String>,
    Form(updated): Form<UpdatedTournament>,
) -> HttpResult {
    let date = Date::parse(&date, ISO_DATE).expect("Data este invalida");
    let capacity = updated
        .capacity
        .as_ref()
        .and_then(|capacity| capacity.parse::<u8>().ok());

    let mut tx = state.write_pool.begin().await?;

    let Some(current) = get_tournament_day(&mut *tx, date).await? else {
        return Ok(error_bubble_response("Nu exista acest turneu"));
    };

    let consumes_reservation = updated.consumes_reservation == Some("on".to_string());

    query!(
        "update alternative_days
          set description = $2, slots_start_hour = $3, slots_start_minute = $4, slot_duration = $5,
           slot_capacity = $6, consumes_reservation = $7
          where date = $1",
        date,
        updated.description,
        updated.start_hour,
        updated.start_minute,
        updated.duration,
        capacity,
        consumes_reservation
    )
    .execute(&mut *tx)
    .await?;

    info!("Tournament at date {date} was updated: {updated:?}");

    if current.start_hour != updated.start_hour {
        let rows_affected = query!(
            "update reservations set hour = $3 where date = $1 and hour = $2",
            date,
            current.start_hour,
            updated.start_hour
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        info!("{rows_affected} reservations were updated when changing tournament start hour");
    }

    tx.commit().await?;

    Ok([("HX-Redirect", "/admin/tournaments")].into_response())
}

pub async fn delete_tournament(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> HttpResult {
    delete_alternative_day(&state, date).await?;
    Ok(().into_response())
}
