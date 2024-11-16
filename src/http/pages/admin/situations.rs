use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::{get_user_reservations, local_time};
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use sqlx::{query, query_as};
use crate::model::user_reservation::UserReservation;
use crate::utils::date_formats;

use crate::utils::date_formats::{ISO_DATE_UNDERLINE, READABLE_DATE_TIME};
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/member", get(members_situation_page))
        .route("/member/:id", get(member_situations))
        .route("/download", get(download_situations))
}

async fn members_situation_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    struct SituationMember {
        email: String,
        name: String,
    }

    #[derive(Template)]
    #[template(path = "pages/admin/situations/member.html")]
    struct MemberSituationTemplate {
        user: User,
        members: Vec<SituationMember>,
    }

    let users = query_as!(SituationMember, "select email, name from users")
        .fetch_all(&state.read_pool)
        .await
        .expect("Database error");

    MemberSituationTemplate {
        user: auth_session.user.expect("User should be logged in"),
        members: users,
    }
}

async fn member_situations(
    Path(email): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "components/reservations_list.html")]
    struct UserReservationsTemplate {
        reservations: Vec<UserReservation>,
        allow_reservation_cancellation: bool,
    }
    
    UserReservationsTemplate {
        reservations: get_user_reservations(&state.read_pool, &email, false).await,
        allow_reservation_cancellation: false
    }
}

async fn download_situations(State(state): State<AppState>) -> impl IntoResponse {
    let current_date = local_time().date().format(ISO_DATE_UNDERLINE).unwrap();

    let mut situations: Vec<_> = query!(
        "select r.*, u.name from reservations r join users u on r.user_id = u.id order by date, hour, created_at"
    )
        .fetch_all(&state.read_pool)
        .await
        .expect("Database error")
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
                res.created_at.format(READABLE_DATE_TIME).unwrap()
            )
        })
        .collect();

    situations.insert(
        0,
        "User ID, Nume, Data, Ora, Ca invitat, Anulat, Creat pentru, Creat pe".to_string(),
    );

    let csv = situations.join("\n");

    Response::builder()
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"situatie_{current_date}.csv\""),
        )
        .body(csv)
        .expect("Failed to create response")
}
