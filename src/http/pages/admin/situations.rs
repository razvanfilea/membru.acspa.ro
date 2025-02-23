use crate::http::AppState;
use crate::http::error::HttpResult;
use crate::http::pages::AuthSession;
use crate::http::pages::home::socket::HoursTemplate;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::utils::date_formats;
use crate::utils::date_formats::{ISO_DATE_UNDERLINE, format_as_local};
use crate::utils::local_time;
use crate::utils::queries::{GroupedUserReservations, get_user_reservations};
use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};
use time::Date;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/member", get(members_situation_page))
        .route("/member", post(member_situations))
        .route("/daily", get(daily_situation_page))
        .route("/daily", post(daily_situation_choose_date))
        .route("/download", get(download_situations))
}

async fn members_situation_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> HttpResult {
    struct SituationMember {
        email: String,
        name: String,
    }

    #[derive(Template)]
    #[template(path = "admin/situations/member_page.html")]
    struct MemberSituationTemplate {
        user: User,
        members: Vec<SituationMember>,
    }

    let users = query_as!(
        SituationMember,
        "select email, name from users order by name"
    )
    .fetch_all(&state.read_pool)
    .await?;

    MemberSituationTemplate {
        user: auth_session.user.expect("User should be logged in"),
        members: users,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct MemberSituationQuery {
    email: String,
}

async fn member_situations(
    State(state): State<AppState>,
    Form(form): Form<MemberSituationQuery>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/reservations_list.html")]
    struct UserReservationsTemplate {
        reservations: Vec<GroupedUserReservations>,
        allow_reservation_cancellation: bool,
    }

    UserReservationsTemplate {
        reservations: get_user_reservations(&state.read_pool, &form.email, false).await,
        allow_reservation_cancellation: false,
    }
    .into_response()
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
    let current_date = local_time().date().format(ISO_DATE_UNDERLINE).unwrap();

    let mut situations: Vec<_> = query!(
        "select r.*, u.name from reservations r join users u on r.user_id = u.id order by date, hour, created_at"
    )
        .fetch_all(&state.read_pool)
        .await?
        .into_iter()
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
                format_as_local(&res.created_at)
            )
        })
        .collect();

    situations.insert(
        0,
        "User ID, Nume, Data, Ora, Ca invitat, Anulat, Creat pentru, Creat pe".to_string(),
    );

    let csv = situations.join("\n");

    Ok(Response::builder()
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"situatie_{current_date}.csv\""),
        )
        .body(csv)?
        .into_response())
}
