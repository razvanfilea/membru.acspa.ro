use crate::utils::date_formats;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use serde::Deserialize;
use sqlx::{query_as, SqlitePool};
use tracing::error;

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::reservation::Reservation;
use crate::model::user::UserUi;

async fn user_reservation(pool: &SqlitePool, email: &str, cancelled: bool) -> Vec<Reservation> {
    query_as!(
        Reservation,
        "select r.* from reservations as r inner join users on user_id = users.id where email = $1 and cancelled = $2 order by date desc, hour asc",
        email,
        cancelled
    ).fetch_all(pool)
        .await
        .inspect_err(|e| error!("Failed querying reservations for user: {e}"))
        .unwrap_or_default()
}

pub async fn profile_page(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/user/profile.html")]
    struct ProfileTemplate<'a> {
        user: UserUi,
        location_name: &'a str,
        duration: i64,
        reservations: Vec<Reservation>,
        show_cancelled: bool,
    }

    let user = auth_session.user.expect("User should be logged in");

    ProfileTemplate {
        location_name: state.location.name.as_ref(),
        duration: state.location.slot_duration,
        reservations: user_reservation(&state.pool, user.email.as_str(), false).await,
        user,
        show_cancelled: false,
    }.into_response()
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
    struct ProfileTemplate<'a> {
        location_name: &'a str,
        duration: i64,
        reservations: Vec<Reservation>,
        show_cancelled: bool,
    }

    let user = auth_session.user.expect("User should be logged in");

    ProfileTemplate {
        location_name: state.location.name.as_ref(),
        duration: state.location.slot_duration,
        reservations: user_reservation(&state.pool, user.email.as_str(), query.show_cancelled).await,
        show_cancelled: query.show_cancelled,
    }.into_response()
}
