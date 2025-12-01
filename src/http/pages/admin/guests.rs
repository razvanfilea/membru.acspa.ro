use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::pages::notification_template::error_bubble_response;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::reservation;
use crate::utils::queries::get_day_structure;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{SqlitePool, query_as};
use time::{Date, OffsetDateTime};
use tracing::{error, info};

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

async fn get_guests(pool: &SqlitePool) -> Result<Vec<GuestDto>, sqlx::Error> {
    query_as!(
        GuestDto,
        r#"select r._rowid_ as 'rowid!', r.created_for 'name!', r.date, r.hour, r.as_guest, r.created_at, r.user_id as created_by_id, u.name as created_by
        from reservations r
        inner join users u on r.user_id = u.id
        where r.created_for is not null
        order by date desc, hour, created_at desc"#
    )
    .fetch_all(pool)
    .await
}

async fn guests_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/guests/guests_page.html")]
    struct GuestsTemplate {
        user: User,
        current_date: Date,
        guests: Vec<GuestDto>,
    }

    GuestsTemplate {
        user: auth_session.user.expect("User should be logged in"),
        guests: get_guests(&state.read_pool).await?,
        current_date: local_time().date(),
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct SelectDateForm {
    date: String,
}

async fn select_hour(
    State(state): State<AppState>,
    Form(form): Form<SelectDateForm>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/guests/select_hour.html")]
    struct SelectHourTemplate {
        hours: Vec<u8>,
    }

    let Ok(date) = Date::parse(&form.date, date_formats::ISO_DATE) else {
        return Ok(error_bubble_response("Data selectata este invalidă"));
    };

    let day_structure = get_day_structure(&state, date).await;

    SelectHourTemplate {
        hours: day_structure.iter().collect(),
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct NewGuestForm {
    name: String,
    date: String,
    hour: u8,
    special: Option<String>,
}

async fn create_guest(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(guest): Form<NewGuestForm>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/guests/guests_page.html", block = "list")]
    struct GuestsListTemplate {
        guests: Vec<GuestDto>,
    }

    let date = Date::parse(&guest.date, date_formats::ISO_DATE).unwrap();
    let day_structure = get_day_structure(&state, date).await;
    if !day_structure.is_hour_valid(guest.hour) {
        error!("Invalid hour: {} for date: {}", guest.hour, guest.date);

        return GuestsListTemplate {
            guests: get_guests(&state.read_pool).await?,
        }
        .try_into_response();
    }

    let user = auth_session.user.expect("User should be logged in");
    let name = guest.name.trim();
    let special = guest.special.is_some();
    let hour = guest.hour;

    let tx = state.write_pool.begin().await?;
    reservation::create_referred_guest(tx, &state.location, date, hour, user.id, special, name)
        .await?;

    info!(
        "Add special guest with date: {date} hour: {} and name: {name}",
        guest.hour
    );

    let _ = state.reservation_notifier.send(());

    GuestsListTemplate {
        guests: get_guests(&state.read_pool).await?,
    }
    .try_into_response()
}
