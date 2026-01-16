use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::model::user_reservation::{GroupedUserReservations, ReservationsCount};
use crate::utils::date_formats::DateFormatExt;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Query, State};
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
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;

    let role = query!(
        "select reservations, guest_reservations from user_roles where id = $1",
        user.role_id
    )
    .fetch_one(&state.read_pool)
    .await?;

    let this_weeks_reservations =
        ReservationsCount::fetch_user_week(&state.read_pool, &user, local_time().date()).await?;

    ProfileTemplate {
        reservations: GroupedUserReservations::fetch_for_user(&state.read_pool, user.id, false)
            .await?,
        user,
        show_cancelled: false,
        this_weeks_reservations,
        max_reservations: ReservationsCount {
            member: role.reservations,
            guest: role.guest_reservations,
        },
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
        reservations: Vec<GroupedUserReservations>,
        show_cancelled: bool,
    }

    let user = auth_session.user.ok_or(HttpError::Unauthorized)?;

    ProfileTemplate {
        reservations: GroupedUserReservations::fetch_for_user(
            &state.read_pool,
            user.id,
            query.show_cancelled,
        )
        .await?,
        show_cancelled: query.show_cancelled,
    }
    .try_into_response()
}
