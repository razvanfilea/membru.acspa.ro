use crate::utils::date_formats;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use serde::Deserialize;
use sqlx::query_as;
use tracing::error;

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::reservation::Reservation;
use crate::model::user::UserUi;

pub async fn profile_page(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/profile.html")]
    struct ProfileTemplate {
        user: UserUi,
        location_name: String,
        duration: i64,
        reservations: Vec<Reservation>,
        cancelled: bool,
    }

    let user = auth_session.user.unwrap();
    let reservations = query_as!(Reservation, "select r.* from reservations as r inner join users on user_id = users.id where email = $1 and cancelled = false", user.email)
        .fetch_all(&state.pool).await.unwrap_or_else(|e| {
        error!("Failed querying reservations for user: {e}");
        Vec::default()
    });

    ProfileTemplate {
        user: user.into(),
        location_name: state.location.name.clone(),
        duration: state.location.slot_duration,
        reservations,
        cancelled: false,
    }
}

#[derive(Deserialize)]
pub struct ReservationsQuery {
    cancelled: bool,
}

pub async fn profile_reservations(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Query(query): Query<ReservationsQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/profile_content.html")]
    struct ProfileTemplate {
        location_name: String,
        duration: i64,
        reservations: Vec<Reservation>,
        cancelled: bool,
    }

    let user = auth_session.user.unwrap();
    let reservations = query_as!(
        Reservation,
        "select r.* from reservations as r inner join users on user_id = users.id where email = $1 and cancelled = $2", 
        user.email,
        query.cancelled
    ).fetch_all(&state.pool)
        .await
        .inspect_err(|e| error!("Failed querying reservations for user: {e}"))
        .unwrap_or_default();

    ProfileTemplate {
        location_name: state.location.name.clone(),
        duration: state.location.slot_duration,
        reservations,
        cancelled: query.cancelled,
    }
}
