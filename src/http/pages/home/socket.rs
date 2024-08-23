use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use serde::de::IgnoredAny;
use serde::Deserialize;
use time::Date;
use tokio::select;
use tracing::{error, warn};
use crate::http::AppState;
use crate::http::pages::home::DAYS_AHEAD_ALLOWED;
use crate::http::pages::home::reservation_hours::{get_reservation_hours, ReservationsSlot};
use crate::utils::date_iter::DateIter;
use crate::utils::{date_formats, local_time};
use crate::utils::CssColor;


pub async fn ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

#[derive(Deserialize)]
struct WsMessage {
    selected_date: String,
    #[serde(rename = "HEADERS")]
    _headers: IgnoredAny,
}

impl WsMessage {
    fn parse(message: Option<Result<Message, axum::Error>>) -> Option<Self> {
        let message = match message {
            Some(Ok(message)) => message,
            Some(Err(e)) => {
                warn!("Socket closed: {e}");
                return None;
            }
            None => return None,
        };

        match message {
            Message::Text(text) => serde_json::from_str::<Self>(text.as_str())
                .inspect_err(|e| warn!("Failed to parse WebSocket message {text} with error: {e}"))
                .ok(),
            _ => None,
        }
    }
}

#[derive(Template)]
#[template(path = "components/home/content.html")]
struct HomeContentTemplate {
    current_date: Date,
    selected_date: Date,
    days: DateIter,
    reservation_hours: Vec<ReservationsSlot>,
}

#[derive(Template)]
#[template(path = "components/home/hours.html")]
struct HoursTemplate {
    reservation_hours: Vec<ReservationsSlot>,
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut selected_date = local_time().date();

    let mut reservations_changed = state.reservation_notifier.subscribe();
    loop {
        let reservations_task = reservations_changed.changed();
        let recv_task = socket.recv();

        let current_date = local_time().date();
        let response = select! {
            result = reservations_task => {
                if let Err(e) = result {
                    error!("Watcher closed unexpectedly: {e}");
                    return;
                }

                // Only send update if something changed on this day
                if *reservations_changed.borrow_and_update() != selected_date {
                    continue;
                }

                HoursTemplate {
                    reservation_hours: get_reservation_hours(&state, selected_date).await,
                }
                .to_string()
            }
            message = recv_task => {
                let Some(ws_message) = WsMessage::parse(message) else {
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

                HomeContentTemplate {
                    current_date,
                    selected_date,
                    days: DateIter::weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
                    reservation_hours: get_reservation_hours(&state, selected_date).await,
                }
                .to_string()
            }
        };

        socket.send(Message::Text(response)).await.unwrap();
    }
}

