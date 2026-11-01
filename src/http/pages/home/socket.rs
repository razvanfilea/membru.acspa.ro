use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::debtors::check_user_has_paid;
use crate::http::pages::home::panels::{CalendarTemplate, DayPanelTemplate, clamp_date};
use crate::http::pages::home::reservation_hours::ReservationHours;
use crate::http::pages::notification_template::NotificationBubbleResponse;
use crate::model::user::User;
use crate::utils::date_formats::{DateFormatExt, IsoDate};
use crate::utils::{CssColor, local_date};
use askama::Template;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use itertools::Itertools;
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
    let user = auth_session.user().await.ok_or(HttpError::Unauthorized)?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user)))
}

#[derive(Deserialize)]
struct WsMessage {
    selected_date: IsoDate,
    /// Sent by the browser after the socket (re)connects, to bring the page back in sync
    #[serde(default)]
    resync: bool,
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

async fn handle_socket(mut socket: WebSocket, state: AppState, mut user: User) {
    let mut selected_date = local_date();
    let mut reservations_changed = state.reservation_notifier.subscribe();

    if user.role == "Admin" {
        let current_date = local_date();
        let celebrated = query!(
            "select name, received_gift from users where is_deleted = false and strftime('%d%m', birthday) = strftime('%d%m', $1)",
            current_date
        )
        .fetch_all(&state.read_pool)
        .await.unwrap_or_default();

        let message = celebrated
            .into_iter()
            .map(|user| {
                let gift = if let Some(gift_date) = user.received_gift {
                    format!(", a primit cadou pe {}", gift_date.to_readable())
                } else {
                    " și nu a primit cadou!!".to_string()
                };

                format!("Este ziua lui {}{}", user.name, gift)
            })
            .join("<br><br>");
        if !message.is_empty() {
            let _ = socket
                .send(Message::Text(
                    NotificationBubbleResponse {
                        message: message.as_str(),
                    }
                    .to_string()
                    .into(),
                ))
                .await;
        }
    }

    // The page only connects once the user has paid, and shows the hours to active users
    let mut showing_hours = user.is_active;

    loop {
        let reservations_task = reservations_changed.changed();
        let recv_task = socket.recv();

        let response = select! {
            result = reservations_task => {
                if let Err(e) = result {
                    error!("Watcher closed unexpectedly: {e}");
                    return;
                }

                reservations_changed.borrow_and_update();

                let today = local_date();
                let clamped = clamp_date(selected_date, today);
                let Some((fresh_user, has_paid)) = refresh_user(&state, &user).await else {
                    continue;
                };
                user = fresh_user;

                // The calendar is replaced too, its counts and the week window may have changed.
                // The whole panel is only replaced when something other than the hours changed,
                // so a confirmation prompt that is open stays
                let can_reserve = user.is_active && has_paid;
                let Ok(calendar) = CalendarTemplate::render(&state, today, clamped).await else {
                    error!("Database error fetching the calendar");
                    continue;
                };
                let panel = if clamped != selected_date || can_reserve != showing_hours {
                    DayPanelTemplate::render(&state, clamped, &user, has_paid).await
                } else if can_reserve {
                    HoursTemplate::create_response(&state, clamped, &user, true).await
                } else {
                    Some(String::new())
                };
                let Some(panel) = panel else {
                    continue;
                };

                selected_date = clamped;
                showing_hours = can_reserve;
                calendar + &panel
            }
            message = recv_task => {
                let Some(ws_message) = WsMessage::parse(message) else {
                    return;
                };

                let today = local_date();
                let requested = *ws_message.selected_date;
                selected_date = clamp_date(requested, today);

                let Some((fresh_user, has_paid)) = refresh_user(&state, &user).await else {
                    continue;
                };
                user = fresh_user;

                let Some(panel) = DayPanelTemplate::render(&state, selected_date, &user, has_paid).await else {
                    continue;
                };
                showing_hours = user.is_active && has_paid;

                // The clicked day is highlighted by the browser, so the calendar is only needed to
                // put back an invalid date, or after a reconnect when it may have missed changes
                if selected_date == requested && !ws_message.resync {
                    panel
                } else {
                    match CalendarTemplate::render(&state, today, selected_date).await {
                        Ok(calendar) => calendar + &panel,
                        Err(e) => {
                            error!("Database error fetching the calendar: {e}");
                            panel
                        }
                    }
                }
            }
        };

        if socket.send(Message::Text(response.into())).await.is_err() {
            return;
        }
    }
}

/// Reads the account again, so a deactivation or a missed payment applies without reconnecting
async fn refresh_user(state: &AppState, user: &User) -> Option<(User, bool)> {
    let user = User::fetch_by_email(&state.read_pool, &user.email)
        .await
        .inspect_err(|e| error!("Database error fetching user {}: {e}", user.id))
        .ok()??;
    let has_paid = check_user_has_paid(&state.read_pool, &user)
        .await
        .inspect_err(|e| error!("Database error checking payments of user {}: {e}", user.id))
        .ok()?;

    Some((user, has_paid))
}
