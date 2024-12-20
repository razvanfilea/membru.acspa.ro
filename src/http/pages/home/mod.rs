use crate::http::pages::home::reservation_hours::{get_reservation_hours, ReservationHours};
use crate::http::pages::home::socket::handle_ws;
use crate::http::pages::{get_global_vars, AuthSession};
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::user::User;
use crate::reservation::{
    create_reservation, is_reservation_possible, ReservationError, ReservationSuccess,
};
use crate::utils::date_iter::DateIter;
use crate::utils::queries::get_hour_structure_for_day;
use crate::utils::CssColor;
use crate::utils::{date_formats, get_reservation_result_color, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;
use time::Date;
use tracing::{error, warn};

pub mod reservation_hours;
pub mod socket;

const DAYS_AHEAD_ALLOWED: time::Duration = time::Duration::days(14);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/ws", get(handle_ws))
        .route("/choose_hour", post(hour_picker))
        .route("/reservation", post(confirm_reservation))
        .route("/reservation", delete(cancel_reservation))
}

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[derive(Template)]
    #[template(path = "pages/home.html")]
    struct HomeTemplate {
        current_date: Date,
        selected_date: Date,
        days: DateIter,
        user: User,
        reservation_hours: ReservationHours,
        global_vars: GlobalVars,
    }

    let current_date = local_time().date();

    HomeTemplate {
        current_date,
        selected_date: current_date,
        days: DateIter::weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
        user: auth_session.user.expect("User should be logged in"),
        reservation_hours: get_reservation_hours(&state, current_date).await,
        global_vars: get_global_vars(&state).await,
    }
}

#[derive(Deserialize)]
struct HourQuery {
    selected_date: String,
    hour: u8,
}

#[derive(Template)]
#[template(path = "components/home/reservation_confirmed.html")]
struct ConfirmedTemplate {
    successful: bool,
    message_color: CssColor,
    message: String,
}

async fn hour_picker(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/home/reservation_confirm_card.html")]
    struct ConfirmationPromptTemplate<'a> {
        selected_date: Date,
        start_hour: u8,
        end_hour: u8,
        location_name: &'a str,
    }

    let user = auth_session.user.expect("User should be logged in");
    let selected_date = Date::parse(&query.selected_date, date_formats::READABLE_DATE)
        .unwrap_or_else(|e| {
            warn!(
                "Failed to parse date {} with error: {}",
                query.selected_date, e
            );
            local_time().date()
        });

    let structure = get_hour_structure_for_day(&state, selected_date).await;

    let mut tx = state
        .read_pool
        .begin()
        .await
        .expect("Failed to create transaction");

    let is_possible = is_reservation_possible(
        tx.as_mut(),
        &state.location,
        local_time(),
        &user,
        selected_date,
        query.hour,
    )
    .await;

    if let Err(e) = is_possible.as_ref() {
        ConfirmedTemplate {
            message: e.to_string(),
            message_color: get_reservation_result_color(&is_possible),
            successful: false,
        }
        .into_response()
    } else {
        ConfirmationPromptTemplate {
            selected_date,
            start_hour: query.hour,
            end_hour: query.hour + structure.slot_duration as u8,
            location_name: state.location.name.as_ref(),
        }
        .into_response()
    }
}

async fn confirm_reservation(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> impl IntoResponse {
    let user = auth_session.user.expect("User should be logged in");
    let selected_date =
        Date::parse(&query.selected_date, date_formats::READABLE_DATE).expect("Invalid date");
    let selected_hour = query.hour;

    let result = create_reservation(
        &state.write_pool,
        &state.location,
        local_time(),
        &user,
        selected_date,
        selected_hour,
    )
    .await;

    let message = match result.as_ref() {
        Ok(success) => {
            let _ = state.reservation_notifier.send(());

            match success {
                ReservationSuccess::Reservation{ .. } => format!(
                    "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
                ReservationSuccess::Guest => format!(
                    "Ai fost înscris ca invitat pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
                ReservationSuccess::InWaiting{as_guest} => format!(
                    "Ești in așteptare{} pentru data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    if *as_guest { " ca și invitat"} else {""},
                    query.selected_date
                ),
            }
        }
        Err(e) => {
            if let ReservationError::DatabaseError(e) = &e {
                error!("Database error when creating reservation on {selected_date} hour {selected_hour} for user {}: {e}", user.email);
            }
            e.to_string()
        }
    };

    ConfirmedTemplate {
        successful: result.is_ok(),
        message_color: get_reservation_result_color(&result),
        message,
    }
}

#[derive(Deserialize)]
struct CancelReservationQuery {
    date: String,
    hour: u8,
    user_id: Option<i64>,
    created_for: Option<String>,
}

async fn cancel_reservation(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Query(query): Query<CancelReservationQuery>,
) -> impl IntoResponse {
    let date = Date::parse(&query.date, date_formats::ISO_DATE).unwrap();
    let user = auth_session.user.expect("User should be logged in");
    let user_id = query.user_id.unwrap_or(user.id);

    if (user_id != user.id || query.created_for.is_some()) && !user.admin_panel_access {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut tx = state.write_pool.begin().await.expect("Database error");
    let rows = if let Some(created_for) = query.created_for {
        query!("delete from reservations where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for = $5", 
            date, query.hour, user_id, state.location.id, created_for)
            .execute(tx.as_mut())
            .await
    } else {
        query!("update reservations set cancelled = true
        where date = $1 and hour = $2 and user_id = $3 and location = $4 and created_for is null",
            date, query.hour, user_id, state.location.id)
            .execute(tx.as_mut())
            .await
    }

        .expect("Database error")
        .rows_affected();
    if rows != 1 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let count = query!(
        "select count(*) as 'count!' from reservations where
            date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = false",
        date,
        query.hour,
        state.location.id
    )
    .fetch_one(tx.as_mut())
    .await
    .expect("Database error")
    .count;

    if count < state.location.slot_capacity {
        query!(
            "update reservations set in_waiting = false where rowid =
                (select rowid from reservations where
                    date = $1 and hour = $2 and location = $3 and cancelled = false and in_waiting = true
                    order by as_guest, created_at limit 1)",
            date, query.hour, state.location.id)
        .execute(tx.as_mut())
        .await
        .expect("Database error");
    }

    tx.commit().await.expect("Database error");

    let _ = state.reservation_notifier.send(());

    ().into_response()
}
