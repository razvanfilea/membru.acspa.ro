use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::http::AppState;
use crate::model::user::User;
use crate::model::user_reservation::UserReservation;
use crate::utils::queries::{
    get_user_reservations, get_user_weeks_reservations_count, ReservationsCount,
};
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::query;

pub async fn profile_page(auth_session: AuthSession, State(state): State<AppState>) -> HttpResult {
    #[derive(Template)]
    #[template(path = "pages/user/profile.html")]
    struct ProfileTemplate {
        user: User,
        reservations: Vec<UserReservation>,
        show_cancelled: bool,
        this_weeks_reservations: ReservationsCount,
        max_reservations: ReservationsCount,
    }

    let user = auth_session.user.expect("User should be logged in");

    let role = query!(
        "select reservations, guest_reservations from user_roles where id = $1",
        user.role_id
    )
    .fetch_one(&state.read_pool)
    .await?;

    let this_weeks_reservations =
        get_user_weeks_reservations_count(&state.read_pool, &user, local_time().date()).await?;

    Ok(ProfileTemplate {
        reservations: get_user_reservations(&state.read_pool, user.email.as_str(), false).await,
        user,
        show_cancelled: false,
        this_weeks_reservations,
        max_reservations: ReservationsCount {
            member: role.reservations,
            guest: role.guest_reservations,
        },
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct ReservationsQuery {
    show_cancelled: bool,
}

pub async fn profile_reservations(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Query(query): Query<ReservationsQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/profile_content.html")]
    struct ProfileTemplate {
        reservations: Vec<UserReservation>,
        show_cancelled: bool,
    }

    let user = auth_session.user.expect("User should be logged in");

    ProfileTemplate {
        reservations: get_user_reservations(
            &state.read_pool,
            user.email.as_str(),
            query.show_cancelled,
        )
        .await,
        show_cancelled: query.show_cancelled,
    }
    .into_response()
}
