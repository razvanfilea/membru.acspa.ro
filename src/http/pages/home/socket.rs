use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::pages::home::DAYS_AHEAD_ALLOWED;
use crate::http::pages::home::reservation_hours::ReservationHours;
use crate::http::pages::notification_template::NotificationBubbleResponse;
use crate::model::user::User;
use crate::utils::date_formats::{DateFormatExt, IsoDate};
use crate::utils::dates::DateRangeIter;
use crate::utils::{CssColor, local_date};
use askama::Template;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use serde::Deserialize;
use serde::de::IgnoredAny;
use sqlx::query;
use time::Date;
use tokio::select;
use tracing::{debug, error, warn};

pub async fn handle_ws(
    State(state): State<AppState>,
    auth_session: AuthSession,
    ws: WebSocketUpgrade,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user)))
}

#[derive(Deserialize)]
struct WsMessage {
    selected_date: IsoDate,
    #[serde(rename = "HEADERS")]
    _headers: IgnoredAny,
}

impl WsMessage {
    fn parse(message: Option<Result<Message, axum::Error>>) -> Option<Self> {
        let message = match message {
            Some(Ok(message)) => message,
            Some(Err(e)) => {
                debug!("Socket closed: {e}");
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
#[template(path = "home/content.html")]
struct HomeContentTemplate<'a> {
    current_date: Date,
    selected_date: Date,
    days: DateRangeIter,
    reservation_hours: ReservationHours,
    user: &'a User,
    has_paid: bool,
}

#[derive(Template)]
#[template(path = "home/hours.html")]
pub struct HoursTemplate<'a> {
    reservation_hours: ReservationHours,
    selected_date: Date,
    user: &'a User,
    enable_editing: bool,
}

impl<'a> HoursTemplate<'a> {
    pub async fn create_response(
        state: &AppState,
        selected_date: Date,
        user: &'a User,
        enable_editing: bool,
    ) -> Option<String> {
        let reservation_hours = ReservationHours::fetch(state, selected_date)
            .await
            .inspect_err(|e| error!("Database error fetching reservation hours: {e}"))
            .ok()?;

        Some(
            Self {
                reservation_hours,
                selected_date,
                user,
                enable_editing,
            }
            .to_string(),
        )
    }
}

async fn handle_socket(mut socket: WebSocket, state: AppState, user: User) {
    let mut selected_date = local_date();
    let mut reservations_changed = state.reservation_notifier.subscribe();

    if user.role == "Admin" {
        let current_date = local_date();
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
                        gift_date.to_readable()
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
                        .to_string().into(),
                    ))
                    .await;
            }
        }
    }

    loop {
        let reservations_task = reservations_changed.changed();
        let recv_task = socket.recv();

        let current_date = local_date();
        let response = select! {
            result = reservations_task => {
                if let Err(e) = result {
                    error!("Watcher closed unexpectedly: {e}");
                    return;
                }

                reservations_changed.borrow_and_update();

                let Some(response) = HoursTemplate::create_response(&state, selected_date, &user, true).await else {
                    continue;
                };
                response
            }
            message = recv_task => {
                let Some(ws_message) = WsMessage::parse(message) else {
                    return;
                };

                let parsed_date = *ws_message.selected_date;
                selected_date = if parsed_date >= current_date && parsed_date <= current_date + DAYS_AHEAD_ALLOWED {
                    parsed_date
                } else {
                    current_date
                };

                let Ok(reservation_hours) = ReservationHours::fetch(&state, selected_date).await else {
                    error!("Database error fetching reservation hours");
                    continue;
                };

                HomeContentTemplate {
                    current_date,
                    selected_date,
                    days: DateRangeIter::weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
                    reservation_hours,
                    user: &user,
                    has_paid: true,
                }
                .to_string()
            }
        };

        if socket.send(Message::Text(response.into())).await.is_err() {
            return;
        }
    }
}
