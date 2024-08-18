use crate::utils::{date_formats, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Query, State};
use serde::Deserialize;
use sqlx::{query_as, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::error;

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::UserUi;

struct UserReservation {
    pub date: Date,
    pub hour: i64,

    pub as_guest: bool,

    pub cancelled: bool,
    pub in_waiting: bool,

    pub created_at: OffsetDateTime,
}

impl UserReservation {
    pub fn is_cancellable(&self) -> bool {
        let now = local_time();
        let now_date = now.date();
        !self.cancelled
            && (self.date > now_date
                || (self.date == now_date && self.hour as u8 >= now.time().hour()))
    }
}

async fn user_reservation(pool: &SqlitePool, email: &str, cancelled: bool) -> Vec<UserReservation> {
    query_as!(
        UserReservation,
        "select r.date, r.hour, r.as_guest, r.cancelled, r.in_waiting, r.created_at from reservations as r inner join users on user_id = users.id where email = $1 and cancelled = $2 and created_for is null order by date desc, hour asc",
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
    struct ProfileTemplate {
        user: UserUi,
        reservations: Vec<UserReservation>,
        show_cancelled: bool,
    }

    let user = auth_session.user.expect("User should be logged in");

    ProfileTemplate {
        reservations: user_reservation(&state.pool, user.email.as_str(), false).await,
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
        reservations: user_reservation(&state.pool, user.email.as_str(), query.show_cancelled)
            .await,
        show_cancelled: query.show_cancelled,
    }
    .into_response()
}
