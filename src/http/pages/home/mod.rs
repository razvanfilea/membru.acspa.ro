use crate::http::pages::home::calendar::{get_weeks_in_range, Weeks};
use crate::http::pages::home::reservation::{create_reservation, ReservationSuccess};
use crate::http::pages::{get_global_vars, AuthSession};
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::restriction::Restriction;
use crate::model::user::UserUi;
use crate::utils::{date_formats, get_hour_structure_for_day, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::de::IgnoredAny;
use serde::Deserialize;
use sqlx::{query, query_as};
use time::Date;
use tokio::select;
use tracing::{error, warn};

mod calendar;
mod reservation;

const DAYS_AHEAD_ALLOWED: time::Duration = time::Duration::days(14);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/ws", get(ws))
        .route("/choose_hour", post(hour_picker))
        .route("/reservation", post(confirm_reservation))
        .route("/reservation", delete(cancel_reservation))
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
                .map(|record| PossibleReservation {
                    name: record.name.clone(),
                    has_key: record.has_key,
                    res_type: ReservationType::Normal,
                });

            let special_guests = date_special_guests
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| PossibleReservation {
                    name: record.name.clone(),
                    has_key: false,
                    res_type: ReservationType::SpecialGuest,
                });

            let guests = date_guests
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| PossibleReservation {
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
    SpecialGuest,
    Guest,
}

struct PossibleReservation {
    name: String,
    has_key: bool,
    res_type: ReservationType,
}

struct PossibleReservationSlot {
    start_hour: u8,
    end_hour: u8,
    reservations: Result<Vec<PossibleReservation>, String>,
}

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/home.html")]
    struct HomeTemplate {
        current_date: Date,
        selected_date: Date,
        weeks: Weeks,
        user: UserUi,
        reservation_hours: Vec<PossibleReservationSlot>,
        global_vars: GlobalVars,
    }

    let current_date = local_time().date();

    HomeTemplate {
        current_date,
        selected_date: current_date,
        weeks: get_weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
        user: auth_session.user.expect("User should be logged in"),
        reservation_hours: get_reservation_hours(&state, current_date).await,
        global_vars: get_global_vars(&state).await,
    }
}

async fn ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

#[derive(Deserialize)]
struct WsMessage {
    selected_date: String,
    #[serde(rename = "HEADERS")]
    _headers: IgnoredAny,
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut selected_date = local_time().date();

    #[derive(Template)]
    #[template(path = "components/home/content.html")]
    struct HomeContentTemplate {
        current_date: Date,
        selected_date: Date,
        weeks: Weeks,
        reservation_hours: Vec<PossibleReservationSlot>,
    }

    fn parse_message(message: Option<Result<Message, axum::Error>>) -> Option<WsMessage> {
        let message = match message {
            Some(Ok(message)) => message,
            Some(Err(e)) => {
                warn!("Socket closed: {e}");
                return None;
            }
            None => return None,
        };

        match message {
            Message::Text(text) => serde_json::from_str::<WsMessage>(text.as_str())
                .inspect_err(|e| warn!("Failed to parse WebSocket message {text} with error: {e}"))
                .ok(),
            _ => None,
        }
    }

    let mut reservations_changed = state.reservation_notifier.subscribe();
    loop {
        let reservations_task = reservations_changed.changed();

        let current_date = local_time().date();
        select! {
            result = reservations_task => {
                if let Err(e) = result {
                    error!("Watcher closed unexpectedly: {e}");
                    return;
                }

                // Only send update if something changed on this day
                if *reservations_changed.borrow_and_update() != selected_date {
                    continue;
                }
            }
            message = socket.recv() => {
                let Some(ws_message) = parse_message(message) else {
                    return;
                };

                selected_date = Date::parse(&ws_message.selected_date, date_formats::ISO_DATE)
                    .inspect_err(|e| {
                        warn!(
                            "Failed to parse date {} with error: {e}",
                            ws_message.selected_date
                        )
                    })
                    .ok()
                    .filter(|date| {
                        date >= &current_date && selected_date <= current_date + DAYS_AHEAD_ALLOWED
                    })
                    .unwrap_or(current_date);
            }
        }

        let response = HomeContentTemplate {
            current_date,
            selected_date,
            weeks: get_weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
            reservation_hours: get_reservation_hours(&state, selected_date).await,
        }
        .to_string();

        socket.send(Message::Text(response)).await.unwrap();
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
    struct ConfirmationTemplate<'a> {
        selected_date: Date,
        start_hour: u8,
        end_hour: u8,
        location_name: &'a str,
    }

    let selected_date = Date::parse(&query.selected_date, date_formats::READABLE_DATE)
        .unwrap_or_else(|e| {
            warn!(
                "Failed to pase date {} with error: {}",
                query.selected_date, e
            );
            local_time().date()
        });

    let structure = get_hour_structure_for_day(&state, &selected_date).await;

    ConfirmationTemplate {
        selected_date,
        start_hour: query.hour,
        end_hour: query.hour + structure.slot_duration as u8,
        location_name: state.location.name.as_ref(),
    }
    .into_response()
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

    let user = auth_session.user.expect("User should be logged in");
    let selected_date =
        Date::parse(&query.selected_date, date_formats::READABLE_DATE).expect("Invalid date");
    let selected_hour = query.hour;

    let result =
        create_reservation(&state, local_time(), &user, selected_date, selected_hour).await;
    let successful = result.is_ok();
    let message = match result {
        Ok(success) => {
            let _ = state.reservation_notifier.send(selected_date);

            match success {
                ReservationSuccess::Reservation => format!(
                    "Ai fost înscris ca invitat pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
                ReservationSuccess::Guest => format!(
                    "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
            }
        }
        Err(e) => e.to_string(),
    };

    ConfirmationTemplate {
        successful,
        message,
    }
}

#[derive(Deserialize)]
struct CancelReservationQuery {
    date: String,
    hour: u8,
}

async fn cancel_reservation(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Query(query): Query<CancelReservationQuery>,
) -> impl IntoResponse {
    let date = Date::parse(&query.date, date_formats::ISO_DATE).unwrap();
    let user = auth_session.user.expect("User should be logged in");

    let cancelled_date = query!(
        "update reservations set cancelled = true where date = $1 and hour = $2 and user_id = $3 and location = $4 returning date",
        date, query.hour, user.id, state.location.id)
        .fetch_optional(&state.pool)
        .await
        .expect("Database error")
        .map(|record| record.date);

    if let Some(date) = cancelled_date {
        let _ = state.reservation_notifier.send(date);
        return ().into_response();
    }

    StatusCode::BAD_REQUEST.into_response()
}
