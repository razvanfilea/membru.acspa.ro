use crate::http::pages::home::reservation_hours::{get_reservation_hours, ReservationsSlot};
use crate::http::pages::home::DAYS_AHEAD_ALLOWED;
use crate::http::pages::notification_template::{
    notification_bubble_response, NotificationBubbleResponse,
};
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::date_formats::READABLE_DATE;
use crate::utils::date_iter::DateIter;
use crate::utils::CssColor;
use crate::utils::{date_formats, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use serde::de::IgnoredAny;
use serde::Deserialize;
use sqlx::{query, query_as};
use time::Date;
use tokio::select;
use tracing::{error, warn};

pub async fn ws(
    State(state): State<AppState>,
    auth_session: AuthSession,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user = auth_session.user.expect("User should be logged in");
    ws.on_upgrade(move |socket| handle_socket(socket, state, user))
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
struct HomeContentTemplate<'a> {
    current_date: Date,
    selected_date: Date,
    days: DateIter,
    reservation_hours: Vec<ReservationsSlot>,
    user: &'a User,
}

#[derive(Template)]
#[template(path = "components/home/hours.html")]
struct HoursTemplate<'a> {
    reservation_hours: Vec<ReservationsSlot>,
    selected_date: Date,
    user: &'a User,
}

async fn handle_socket(mut socket: WebSocket, state: AppState, user: User) {
    let mut selected_date = local_time().date();
    let mut reservations_changed = state.reservation_notifier.subscribe();

    if user.role == "Admin" {
        let current_date = local_time().date();
        if let Ok(celebrated) = query!(
            "select name, received_gift from users where strftime('%d%m', birthday) = strftime('%d%m', $1)",
            current_date
        )
        .fetch_all(&state.read_pool)
        .await
        {
            for user in celebrated {
                let gift = if let Some(gift_date) = user.received_gift {
                    format!(
                        ", a primit cadou pe {}",
                        gift_date.format(READABLE_DATE).expect("Invalid date in DB")
                    )
                } else {
                    " și nu a primit cadou!!".to_string()
                };

                let message = format!("Este ziua lui {}{}", user.name, gift);
                let _ = socket
                    .send(Message::Text(
                        NotificationBubbleResponse {
                            message: message.as_str(),
                        }
                        .to_string(),
                    ))
                    .await;
            }
        }
    }

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

                reservations_changed.borrow_and_update();

                HoursTemplate {
                    reservation_hours: get_reservation_hours(&state, selected_date).await,
                    selected_date,
                    user: &user
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
                    user: &user
                }
                .to_string()
            }
        };

        socket.send(Message::Text(response)).await.unwrap();
    }
}
