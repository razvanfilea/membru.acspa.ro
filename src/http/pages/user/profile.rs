use crate::utils::date_formats;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::User;
use crate::model::user_reservation::UserReservation;
use crate::utils::queries::get_user_reservations;

pub async fn profile_page(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/user/profile.html")]
    struct ProfileTemplate {
        user: User,
        reservations: Vec<UserReservation>,
        show_cancelled: bool,
    }

    let user = auth_session.user.expect("User should be logged in");

    ProfileTemplate {
        reservations: get_user_reservations(&state.read_pool, user.email.as_str(), false).await,
        user,
        show_cancelled: false,
    }
    .into_response()
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
