use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};

use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::role::UserRole;
use crate::model::user::UserUi;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(roles_page))
        .route("/new", get(new_role_page))
        .route("/new", post(create_new_role))
        .route("/edit/:id", get(edit_role_page))
        .route("/edit/:id", post(update_role))
}

async fn roles_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/roles/list.html")]
    struct UsersTemplate {
        user: UserUi,
        roles: Vec<UserRole>,
    }

    let roles = query_as!(UserRole, "select * from user_roles")
        .fetch_all(&state.read_pool)
        .await
        .expect("Database error");

    UsersTemplate {
        user: auth_session.user.expect("User should be logged in"),
        roles,
    }
}

#[derive(Deserialize)]
struct NewRole {
    name: String,
    reservations: i64,
    as_guest: i64,
}

#[derive(Template)]
#[template(path = "pages/admin/roles/new_edit.html")]
struct NewRoleTemplate {
    user: UserUi,
    value: Option<UserRole>,
}

async fn new_role_page(auth_session: AuthSession) -> impl IntoResponse {
    NewRoleTemplate {
        user: auth_session.user.expect("User should be logged in"),
        value: None,
    }
}

async fn create_new_role(
    State(state): State<AppState>,
    Form(role): Form<NewRole>,
) -> impl IntoResponse {
    query!(
        "insert into user_roles (name, max_reservations, max_guest_reservations) VALUES ($1, $2, $3)",
        role.name,
        role.reservations,
        role.as_guest
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/roles")
        .body("Rolul a fost creat cu succes".to_string())
        .expect("Failed to create headers")
}

async fn edit_role_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(role_id): Path<i64>,
) -> impl IntoResponse {
    let role = query_as!(UserRole, "select * from user_roles where id = $1", role_id)
        .fetch_optional(&state.read_pool)
        .await
        .expect("Database error");

    if role.is_none() {
        return Response::builder()
            .header("HX-Redirect", "/admin/roles")
            .body("Rolul nu există".to_string())
            .expect("Failed to create headers")
            .into_response();
    }

    NewRoleTemplate {
        user: auth_session.user.expect("User should be logged in"),
        value: role,
    }
    .into_response()
}

async fn update_role(
    State(state): State<AppState>,
    Path(role_id): Path<i64>,
    Form(role): Form<NewRole>,
) -> impl IntoResponse {
    query!(
        "update user_roles set name = $2, max_reservations = $3, max_guest_reservations = $4 where id = $1",
        role_id,
        role.name,
        role.reservations,
        role.as_guest
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/roles")
        .body("Rolul a fost actualizat cu succes".to_string())
        .expect("Failed to create headers")
}
