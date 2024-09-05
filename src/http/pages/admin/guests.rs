use askama_axum::{IntoResponse, Template};
use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, SqlitePool};
use std::ops::Not;
use time::{Date, OffsetDateTime};
use tracing::{error, info};

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::{date_formats, get_hour_structure_for_day, local_time};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(guests_page))
        .route("/", put(create_guest))
        .route("/select_hour", post(select_hour))
}

pub struct GuestDto {
    rowid: i64,
    name: String,
    date: Date,
    hour: i64,
    as_guest: bool,
    created_by: String,
    created_by_id: i64,
    created_at: OffsetDateTime,
}

async fn get_guests(pool: &SqlitePool) -> Vec<GuestDto> {
    query_as!(
        GuestDto,
        r#"select r._rowid_ as 'rowid!', r.created_for 'name!', r.date, r.hour, r.as_guest, r.created_at, r.user_id as created_by_id, u.name as created_by
        from reservations r
        inner join users u on r.user_id = u.id
        where r.created_for is not null
        order by date desc, hour, created_at"#
    )
    .fetch_all(pool)
    .await
    .expect("Database error")
}

async fn guests_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/guests.html")]
    struct GuestsTemplate {
        user: User,
        current_date: Date,
        guests: Vec<GuestDto>,
    }

    GuestsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        guests: get_guests(&state.read_pool).await,
        current_date: local_time().date(),
    }
}

#[derive(Deserialize)]
struct SelectDateForm {
    date: String,
}

async fn select_hour(
    State(state): State<AppState>,
    Form(form): Form<SelectDateForm>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/select_hour.html")]
    struct SelectHourTemplate {
        hours: Vec<u8>,
    }

    let date = Date::parse(&form.date, date_formats::ISO_DATE).unwrap();

    let hour_structure = get_hour_structure_for_day(&state, date).await;

    SelectHourTemplate {
        hours: hour_structure.iter().collect(),
    }
}

#[derive(Deserialize)]
struct NewSpecialGuest {
    name: String,
    date: String,
    hour: u8,
    special: Option<String>,
}

async fn create_guest(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(guest): Form<NewSpecialGuest>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/guests_content.html")]
    struct GuestsListTemplate {
        guests: Vec<GuestDto>,
    }

    let date = Date::parse(&guest.date, date_formats::ISO_DATE).unwrap();
    let hour_structure = get_hour_structure_for_day(&state, date).await;
    if !hour_structure.is_hour_valid(guest.hour) {
        error!("Invalid hour: {} for date: {}", guest.hour, guest.date);

        return GuestsListTemplate {
            guests: get_guests(&state.read_pool).await,
        };
    }

    let user = auth_session.user.expect("User should be logged in");
    let name = guest.name.trim();
    let special = guest.special.is_some().not();

    query!(
        "insert into reservations (user_id, date, hour, location, created_for, as_guest) VALUES ($1, $2, $3, $4, $5, $6)",
        user.id,
        date,
        guest.hour,
        state.location.id,
        name,
        special
    )
        .execute(&state.write_pool)
        .await
        .expect("Database error");

    info!(
        "Add special guest with date: {date} hour: {} and name: {name}",
        guest.hour
    );

    let _ = state.reservation_notifier.send(());

    GuestsListTemplate {
        guests: get_guests(&state.read_pool).await,
    }
}
