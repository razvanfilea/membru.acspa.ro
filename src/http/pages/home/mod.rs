use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::debtors::check_user_has_paid;
use crate::http::pages::home::reservation_hours::{ReservationHours, get_reservation_hours};
use crate::http::pages::home::socket::handle_ws;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::day_structure::DayStructure;
use crate::model::global_vars::GlobalVars;
use crate::model::user::User;
use crate::reservation;
use crate::reservation::{
    ReservationError, ReservationSuccess, create_reservation, is_reservation_possible,
};
use crate::utils::date_formats::DateFormatExt;
use crate::utils::dates::DateRangeIter;
use crate::utils::{CssColor, local_date};
use crate::utils::{date_formats, get_reservation_result_color, local_time};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;
use std::str::FromStr;
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

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    struct ColorCode {
        name: String,
        color: CssColor,
    }

    #[derive(Template)]
    #[template(path = "home/home_page.html")]
    struct HomeTemplate {
        current_date: Date,
        selected_date: Date,
        days: DateRangeIter,
        user: User,
        reservation_hours: ReservationHours,
        global_vars: GlobalVars,
        reservation_color_code: Vec<ColorCode>,
        has_paid: bool,
    }

    let current_date = local_time().date();

    let reservation_color_code = query!(
        "select color as 'color!', name from user_roles where color is not null and color != 'None'"
    )
    .map(|color_code| ColorCode {
        name: color_code.name,
        color: CssColor::from_str(&color_code.color).unwrap_or(CssColor::None),
    })
    .fetch_all(&state.read_pool)
    .await
    .unwrap_or_default();

    let reservation_hours = get_reservation_hours(&state, current_date).await?;
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let has_paid = check_user_has_paid(&state.read_pool, &user).await?;

    HomeTemplate {
        current_date,
        selected_date: current_date,
        days: DateRangeIter::weeks_in_range(current_date, current_date + DAYS_AHEAD_ALLOWED),
        user,
        reservation_hours,
        global_vars: GlobalVars::fetch(&state.read_pool).await?,
        reservation_color_code,
        has_paid,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct HourQuery {
    selected_date: String,
    hour: u8,
}

#[derive(Template)]
#[template(path = "home/reservation_confirmed.html")]
struct ConfirmedTemplate {
    successful: bool,
    message_color: CssColor,
    message: String,
}

async fn hour_picker(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "home/reservation_confirm_card.html")]
    struct ConfirmationPromptTemplate<'a> {
        selected_date: Date,
        start_hour: u8,
        end_hour: u8,
        location_name: &'a str,
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let selected_date = Date::parse(&query.selected_date, date_formats::READABLE_DATE)
        .unwrap_or_else(|e| {
            warn!(
                "Failed to parse date {} with error: {}",
                query.selected_date, e
            );
            local_date()
        });

    let structure =
        DayStructure::fetch_or_default(&state.read_pool, selected_date, &state.location).await?;

    let mut tx = state.read_pool.begin().await?;

    let is_possible = is_reservation_possible(
        tx.as_mut(),
        &state.location,
        local_time(),
        &user,
        selected_date,
        query.hour,
        None,
    )
    .await;

    if let Err(e) = is_possible.as_ref() {
        ConfirmedTemplate {
            message: e.to_string(),
            message_color: get_reservation_result_color(&is_possible),
            successful: false,
        }
        .try_into_response()
    } else {
        ConfirmationPromptTemplate {
            selected_date,
            start_hour: query.hour,
            end_hour: query.hour + structure.slot_duration as u8,
            location_name: state.location.name.as_ref(),
        }
        .try_into_response()
    }
}

async fn confirm_reservation(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(query): Form<HourQuery>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let selected_date = Date::parse(&query.selected_date, date_formats::READABLE_DATE)
        .or_bail("Data este invalida")?;
    let selected_hour = query.hour;

    let result = create_reservation(
        &state.write_pool,
        &state.location,
        local_time(),
        &user,
        selected_date,
        selected_hour,
        None,
    )
    .await;

    let message = match result.as_ref() {
        Ok(success) => {
            let _ = state.reservation_notifier.send(());

            match success {
                ReservationSuccess::Reservation { .. } => format!(
                    "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
                ReservationSuccess::Guest => format!(
                    "Ai fost înscris ca invitat pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    query.selected_date
                ),
                ReservationSuccess::InWaiting { as_guest } => format!(
                    "Ești in așteptare{} pentru data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
                    if *as_guest { " ca și invitat" } else { "" },
                    query.selected_date
                ),
            }
        }
        Err(e) => {
            if let ReservationError::DatabaseError(e) = &e {
                error!(
                    "Database error when creating reservation on {selected_date} hour {selected_hour} for user {}: {e}",
                    user.email
                );
            }
            e.to_string()
        }
    };

    ConfirmedTemplate {
        successful: result.is_ok(),
        message_color: get_reservation_result_color(&result),
        message,
    }
    .try_into_response()
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
) -> HttpResult {
    let date = Date::parse(&query.date, date_formats::ISO_DATE).or_bail("Data este invalida")?;
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let user_id = query.user_id.unwrap_or(user.id);

    if (user_id != user.id || query.created_for.is_some()) && !user.admin_panel_access {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let tx = state.write_pool.begin().await?;
    let reservation_cancelled = reservation::cancel_reservation(
        tx,
        &state.location,
        date,
        query.hour,
        user_id,
        query.created_for.as_deref(),
    )
    .await?;

    if !reservation_cancelled {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let _ = state.reservation_notifier.send(());

    Ok(().into_response())
}
