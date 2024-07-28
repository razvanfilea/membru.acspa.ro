use askama_axum::{IntoResponse, Template};
use axum::{Form, Router};
use axum::extract::{Path, State};
use axum::routing::{delete, get, post, put};
use serde::Deserialize;
use sqlx::{query, query_as, SqlitePool};
use time::{Date, OffsetDateTime};
use tracing::{error, info};

use crate::http::AppState;
use crate::http::pages::AuthSession;
use crate::model::user::UserUi;
use crate::utils::{date_formats, get_hour_structure_for_day};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(guests_page))
        .route("/", put(create_guest))
        .route("/select_hour", post(select_hour))
        .route("/:id", delete(delete_guest))
}


pub struct SpecialGuestDto {
    rowid: i64,
    name: String,
    date: Date,
    hour: i64,
    created_by: String,
    created_at: OffsetDateTime,
}
async fn get_special_guests(pool: &SqlitePool) -> Vec<SpecialGuestDto> {
    query_as!(
        SpecialGuestDto,
        r#"select g.rowid, g.name, g.date, g.hour, g.created_at, u.name as created_by from special_guests g
        inner join users u on g.created_by = u.id
        order by date desc, hour asc"#
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
    struct RestrictionsTemplate {
        user: UserUi,
        current_date: Date,
        guests: Vec<SpecialGuestDto>,
    }

    RestrictionsTemplate {
        user: auth_session.user.unwrap(),
        guests: get_special_guests(&state.pool).await,
        current_date: OffsetDateTime::now_utc().date()
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

    let hour_structure = get_hour_structure_for_day(&state, &date).await;

    SelectHourTemplate {
        hours: hour_structure.iter().collect(),
    }
}

#[derive(Deserialize)]
struct NewSpecialGuest {
    name: String,
    date: String,
    hour: u8,
}

async fn create_guest(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(guest): Form<NewSpecialGuest>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/admin/guests_content.html")]
    struct GuestsListTemplate {
        guests: Vec<SpecialGuestDto>,
    }

    let date = NaiveDate::parse_from_str(&guest.date, "%Y-%m-%d").unwrap();
        let hour_structure = get_hour_structure_for_day(&state, &date).await;
        if !hour_structure.is_hour_valid(guest.hour) {
            error!("Invalid hour: {} for date: {}", guest.hour, guest.date);

            return GuestsListTemplate {
                guests: get_special_guests(&state.pool).await,
            };
        }

    let user = auth_session.user.unwrap();
    let name = guest.name.trim();

    query!(
        "insert into special_guests (name, date, location, hour, created_by) VALUES ($1, $2, $3, $4, $5)",
        name,
        date,
        state.location.id,
        guest.hour,
        user.id,
    )
        .execute(&state.pool)
        .await
        .expect("Database error");

    info!(
        "Add special guest with date: {date} hour: {} and name: {name}",
        guest.hour
    );

    GuestsListTemplate {
        guests: get_special_guests(&state.pool).await,
    }
}

async fn delete_guest(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    query!("delete from special_guests where rowid = $1", id)
        .execute(&state.pool)
        .await
        .expect("Database error");
}
