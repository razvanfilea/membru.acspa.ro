use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::pages::home::socket::HoursTemplate;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::utils::date_formats;
use crate::utils::local_time;
use askama::Template;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::query;
use time::Date;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/daily", get(daily_situation_page))
        .route("/daily", post(daily_situation_choose_date))
        .route("/download", get(download_situations))
}

async fn daily_situation_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/situations/daily_page.html")]
    struct DailySituationTemplate {
        user: User,
        current_date: Date,
        content: String,
    }

    let current_date = local_time().date();
    let user = auth_session.user.expect("User should be logged in");

    let hours = HoursTemplate::create_response(&state, current_date, &user, false).await;

    DailySituationTemplate {
        user,
        current_date,
        content: hours,
    }
    .into_response()
}

#[derive(Deserialize)]
struct DailySituationQuery {
    date: String,
}

async fn daily_situation_choose_date(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(query): Form<DailySituationQuery>,
) -> impl IntoResponse {
    let date = Date::parse(&query.date, date_formats::ISO_DATE).expect("Failed to parse date");
    let user = auth_session.user.expect("User should be logged in");

    HoursTemplate::create_response(&state, date, &user, false).await
}

async fn download_situations(State(state): State<AppState>) -> HttpResult {
    let current_date = date_formats::as_iso_underline(&local_time().date());

    let mut situations = query!(
        "select r.*, u.name from reservations r join users u on r.user_id = u.id order by date, hour, created_at"
    )
        .map(|res| {
            format!(
                "{}, \"{}\", {}, {}, {}, {}, \"{}\", \"{}\"",
                res.user_id,
                res.name,
                res.date,
                res.hour,
                res.as_guest,
                res.cancelled,
                res.created_for.unwrap_or_default(),
                date_formats::as_local(&res.created_at)
            )
        })
        .fetch_all(&state.read_pool)
        .await?;

    situations.insert(
        0,
        "User ID, Nume, Data, Ora, Ca invitat, Anulat, Creat pentru, Creat pe".to_string(),
    );

    let response = (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"situatie_{current_date}.csv\""),
            ),
        ],
        situations.join("\n"),
    );

    Ok(response.into_response())
}
