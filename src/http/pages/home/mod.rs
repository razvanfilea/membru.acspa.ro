use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};
use time::{Date, OffsetDateTime};
use tracing::warn;

use crate::http::pages::home::calendar::{get_weeks_of_month, MonthDates};
use crate::http::pages::home::reservation::{create_reservation, ReservationSuccess};
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::restriction::Restriction;
use crate::model::user::UserUi;
use crate::utils::{date_formats, get_hour_structure_for_day};

mod calendar;
mod reservation;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/choose_date/", post(date_picker))
        .route("/choose_hour", post(hour_picker))
        .route("/confirm_reservation", post(confirm_reservation))
}

async fn get_global_vars(state: &AppState) -> GlobalVars {
    query_as!(GlobalVars, "select * from global_vars")
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
}

async fn get_reservation_hours(state: &AppState, date: Date) -> Vec<PossibleReservationSlot> {
    let hour_structure = get_hour_structure_for_day(state, &date).await;
    let restrictions = query_as!(
        Restriction,
        "select * from reservations_restrictions where date = $1 order by hour",
        date
    )
    .fetch_all(&state.pool)
    .await
    .expect("Database error");

    // Check if the whole day is restricted
    if let Some(restriction) = restrictions.first().filter(|r| r.hour.is_none()) {
        return hour_structure
            .iter()
            .map(|hour| PossibleReservationSlot {
                start_hour: hour,
                end_hour: hour + hour_structure.slot_duration as u8,
                reservations: Err(restriction.message.clone()),
            })
            .collect();
    }

    let date_reservations = query!("select users.name, hour, has_key from reservations inner join users on user_id = users.id where date = $1 order by created_at", date)
        .fetch_all(&state.pool)
        .await
        .expect("Database error");

    let date_special_guests = query!(
        "select name, hour from special_guests where date = $1 order by created_at",
        date
    )
    .fetch_all(&state.pool)
    .await
    .expect("Database error");

    let date_guests = query!(
        "select u.name, g.hour from guests g inner join users u on g.created_by = u.id where g.date = $1 order by g.created_at",
        date
    )
    .fetch_all(&state.pool)
    .await
    .expect("Database error");

    hour_structure
        .iter()
        .map(|hour| {
            let end_hour = hour + hour_structure.slot_duration as u8;

            if let Some(restriction) = restrictions
                .iter()
                .find(|restriction| restriction.hour == Some(hour as i64))
            {
                return PossibleReservationSlot {
                    start_hour: hour,
                    end_hour,
                    reservations: Err(restriction.message.clone()),
                };
            }

            let reservations = date_reservations
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| Reservation {
                    name: record.name.clone(),
                    has_key: record.has_key,
                    res_type: ReservationType::Normal,
                });

            let special_guests = date_special_guests
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| Reservation {
                    name: record.name.clone(),
                    has_key: false,
                    res_type: ReservationType::SpecialGuest,
                });

            let guests = date_guests
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| Reservation {
                    name: record.name.clone(),
                    has_key: false,
                    res_type: ReservationType::Guest,
                });

            PossibleReservationSlot {
                start_hour: hour,
                end_hour,
                reservations: Ok(reservations.chain(special_guests).chain(guests).collect()),
            }
        })
        .collect()
}

enum ReservationType {
    Normal,
    // Trainer, // TODO Remove
    SpecialGuest,
    Guest,
}

struct Reservation {
    name: String,
    has_key: bool,
    res_type: ReservationType,
}

struct PossibleReservationSlot {
    start_hour: u8,
    end_hour: u8,
    reservations: Result<Vec<Reservation>, String>,
}

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/home.html")]
    struct HomeTemplate {
        current_date: Date,
        selected_date: Date,
        weeks: MonthDates,
        user: UserUi,
        reservation_hours: Vec<PossibleReservationSlot>,
        global_vars: GlobalVars,
    }

    let current_date = OffsetDateTime::now_utc().date();

    HomeTemplate {
        current_date,
        selected_date: current_date,
        weeks: get_weeks_of_month(current_date),
        user: auth_session.user.unwrap(),
        reservation_hours: get_reservation_hours(&state, current_date).await,
        global_vars: get_global_vars(&state).await,
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
        current_date: Date,
        selected_date: Date,
        weeks: MonthDates,
        reservation_hours: Vec<PossibleReservationSlot>,
    }

    let current_date = OffsetDateTime::now_utc().date();
    let selected_date = query
        .selected_date
        .and_then(|date| {
            Date::parse(&date, date_formats::ISO_DATE)
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
        selected_date: Date,
        start_hour: u8,
        end_hour: u8,
        location_name: String,
    }

    let selected_date = Date::parse(&query.selected_date, date_formats::READABLE_DATE)
        .unwrap_or_else(|e| {
            warn!(
                "Failed to pase date {} with error: {}",
                query.selected_date, e
            );
            OffsetDateTime::now_utc().date()
        });

    let structure = get_hour_structure_for_day(&state, &selected_date).await;

    ConfirmationTemplate {
        selected_date,
        start_hour: query.hour,
        end_hour: query.hour + structure.slot_duration as u8,
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
        successful: bool,
        message: String,
    }

    let user = auth_session.user.unwrap();

    let selected_date =
        Date::parse(&query.selected_date, date_formats::READABLE_DATE).expect("Invalid date");

    let selected_hour = query.hour;

    let result = create_reservation(
        &state,
        OffsetDateTime::now_utc(),
        &user,
        selected_date,
        selected_hour,
    )
    .await;
    let successful = result.is_ok();
    let message = match result {
        Ok(success) => match success {
            ReservationSuccess::Reservation => format!("Ai fost înscris ca invitat pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>", query.selected_date),
            ReservationSuccess::Guest => format!(
                "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                query.selected_date
            ),
        },
        Err(e) => e.to_string(),
    };

    ConfirmationTemplate {
        successful,
        message,
    }
}
