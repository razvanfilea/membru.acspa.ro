use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::payments_summary::MonthStatus;
use crate::http::pages::admin::members::payments_summary::{
    MonthStatusView, build_status_grid_response, calculate_year_status,
};
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::user::User;
use crate::model::user_reservation::{GroupedUserReservations, ReservationsCount};
use crate::utils::date_formats::DateFormatExt;
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use sqlx::query;

pub async fn profile_page(auth_session: AuthSession, State(state): State<AppState>) -> HttpResult {
    #[derive(Template)]
    #[template(path = "user/profile_page.html")]
    struct ProfileTemplate {
        user: User,
        reservations: Vec<GroupedUserReservations>,
        show_cancelled: bool,
        this_weeks_reservations: ReservationsCount,
        max_reservations: ReservationsCount,
        current_year: i32,
        selected_year: i32,
        months_status_view: Vec<MonthStatusView>,
        total_paid: i64,
        admin_payments_status_grid: bool,
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;

    let role = query!(
        "select reservations, guest_reservations from user_roles where id = $1",
        user.role_id
    )
    .fetch_one(&state.read_pool)
    .await?;

    let current_date = local_date();
    let this_weeks_reservations =
        ReservationsCount::fetch_user_week(&state.read_pool, &user, current_date).await?;

    let current_year = current_date.year();
    let payments = PaymentWithAllocations::fetch_for_user(&state.read_pool, user.id).await?;
    let breaks = PaymentBreak::fetch_for_user(&state.read_pool, user.id).await?;
    let months_status_view = calculate_year_status(current_year, &user, &payments, &breaks);

    ProfileTemplate {
        reservations: GroupedUserReservations::fetch_for_user_year(
            &state.read_pool,
            user.id,
            false,
            current_year,
        )
        .await?,
        user,
        show_cancelled: false,
        this_weeks_reservations,
        max_reservations: ReservationsCount {
            member: role.reservations,
            guest: role.guest_reservations,
        },
        current_year,
        selected_year: current_year,
        months_status_view,
        total_paid: 0,
        admin_payments_status_grid: false,
    }
    .try_into_response()
}

#[derive(Deserialize)]
pub struct ReservationsQuery {
    show_cancelled: bool,
}

pub async fn profile_reservations(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Query(query): Query<ReservationsQuery>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "user/profile_content.html")]
    struct ProfileTemplate {
        member: User,
        reservations: Vec<GroupedUserReservations>,
        show_cancelled: bool,
        current_year: i32,
        selected_year: i32,
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let current_year = local_date().year();

    ProfileTemplate {
        reservations: GroupedUserReservations::fetch_for_user_year(
            &state.read_pool,
            user.id,
            query.show_cancelled,
            current_year,
        )
        .await?,
        member: user,
        show_cancelled: query.show_cancelled,
        current_year,
        selected_year: current_year,
    }
    .try_into_response()
}

pub async fn profile_reservations_year(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Path(year): Path<i32>,
    Query(query): Query<ReservationsQuery>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "components/reservations_with_year_selector.html")]
    struct ReservationsTemplate {
        member: User,
        reservations: Vec<GroupedUserReservations>,
        allow_reservation_cancellation: bool,
        current_year: i32,
        selected_year: i32,
        show_cancelled: bool,
        admin_reservations_view: bool,
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    let current_year = local_date().year();

    ReservationsTemplate {
        reservations: GroupedUserReservations::fetch_for_user_year(
            &state.read_pool,
            user.id,
            query.show_cancelled,
            year,
        )
        .await?,
        member: user,
        allow_reservation_cancellation: true,
        show_cancelled: query.show_cancelled,
        current_year,
        selected_year: year,
        admin_reservations_view: false,
    }
    .try_into_response()
}

pub async fn payment_status_partial(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Path(year): Path<i32>,
) -> HttpResult {
    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;
    build_status_grid_response(&state.read_pool, user, year, false).await
}
