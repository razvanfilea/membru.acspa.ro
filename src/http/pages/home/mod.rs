use crate::http::pages::home::calendar::{get_weeks_of_month, MonthDates};
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::BasicUser;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Form, Router};
use chrono::Datelike;
use chrono::Utc;
use serde::Deserialize;
use sqlx::query;
use tracing::warn;

mod calendar;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/choose_date/", post(date_picker))
        .route("/choose_hour", post(hour_picker))
        .route("/confirm_reservation", post(confirm_reservation))
}

async fn get_reservation_hours(
    state: &AppState,
    date: chrono::NaiveDate,
) -> Vec<PossibleReservationHour> {
    let mut hours = vec![];
    let location = &state.location;

    for i in 0..location.slots_per_day {
        let hour = location.slots_start_hour + location.slot_duration * i;
        let reservations = query!("select users.name from reservations inner join users on user_id = users.id where date = $1 and hour = $2", date, hour)
            .fetch_all(&state.pool).await.unwrap()
            .into_iter()
            .map(|record| record.name)
            .collect();

        hours.push(PossibleReservationHour {
            start_hour: hour as u8,
            end_hour: (hour + location.slot_duration) as u8,
            reservations,
        })
    }

    hours
}

struct PossibleReservationHour {
    start_hour: u8,
    end_hour: u8,
    reservations: Vec<String>,
}

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/home.html")]
    struct HomeTemplate {
        current_date: chrono::NaiveDate,
        selected_date: chrono::NaiveDate,
        weeks: MonthDates,
        user: BasicUser,
        reservation_hours: Vec<PossibleReservationHour>,
    }

    let current_date = Utc::now().naive_local().date();

    HomeTemplate {
        current_date,
        selected_date: current_date,
        weeks: get_weeks_of_month(current_date),
        user: auth_session.user.unwrap().into(),
        reservation_hours: get_reservation_hours(&state, current_date).await,
    }
}

#[derive(Deserialize)]
struct DateQuery {
    selected_date: Option<String>,
}

async fn date_picker(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/home/content.html")]
    struct HomeContentTemplate {
        current_date: chrono::NaiveDate,
        selected_date: chrono::NaiveDate,
        weeks: MonthDates,
        reservation_hours: Vec<PossibleReservationHour>,
    }

    let current_date = Utc::now().naive_local().date();
    let selected_date = query
        .selected_date
        .and_then(|date| {
            chrono::NaiveDate::parse_from_str(&date, "%d.%m.%Y")
                .inspect_err(|e| warn!("Failed to parse date {date} with error: {e}"))
                .ok()
        })
        .unwrap_or(current_date);

    HomeContentTemplate {
        current_date,
        selected_date,
        weeks: get_weeks_of_month(selected_date),
        reservation_hours: get_reservation_hours(&state, selected_date).await,
    }
}

#[derive(Deserialize)]
struct HourQuery {
    selected_date: String,
    hour: u8,
}

async fn hour_picker(
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/home/reservation_confirm_card.html")]
    struct ConfirmationTemplate {
        selected_date: chrono::NaiveDate,
        start_hour: u8,
        end_hour: u8,
        location_name: String,
    }

    let selected_date = chrono::NaiveDate::parse_from_str(&query.selected_date, "%d.%m.%Y")
        .unwrap_or_else(|e| {
            warn!(
                "Failed to pase date {} with error: {}",
                query.selected_date, e
            );
            Utc::now().naive_local().date()
        });

    ConfirmationTemplate {
        selected_date,
        start_hour: query.hour,
        end_hour: query.hour + state.location.slot_duration as u8,
        location_name: state.location.name,
    }
}

async fn confirm_reservation(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/home/reservation_confirmed.html")]
    struct ConfirmationTemplate {
        selected_date: chrono::NaiveDate,
        start_hour: u8,
        end_hour: u8,
    }

    let user = auth_session.user.unwrap();

    let selected_date =
        chrono::NaiveDate::parse_from_str(&query.selected_date, "%d.%m.%Y").expect("Invalid date");

    query!(
        "insert into reservations (user_id, date, hour, location) VALUES ($1, $2, $3, $4)",
        user.id,
        selected_date,
        query.hour,
        state.location.id
    )
    .execute(&state.pool)
    .await
    .expect("Failed to create reservation");

    ConfirmationTemplate {
        selected_date,
        start_hour: query.hour,
        end_hour: query.hour + state.location.slot_duration as u8,
    }
}
