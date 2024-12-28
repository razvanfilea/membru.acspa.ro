use crate::http::pages::notification_template::error_bubble_response;
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::utils::CssColor;
use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};
use std::str::FromStr;
use strum::IntoEnumIterator;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(roles_page))
        .route("/new", get(new_role_page))
        .route("/new", post(create_new_role))
        .route("/edit/:id", get(edit_role_page))
        .route("/edit/:id", post(update_role))
        .route("/:id", delete(delete_role))
}

async fn roles_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    struct UserRoleWithCount {
        pub id: i64,
        pub name: String,
        pub reservations: i64,
        pub guest_reservations: i64,
        pub color: Option<String>,
        pub admin_panel_access: bool,
        pub members_count: i64,
    }

    #[derive(Template)]
    #[template(path = "pages/admin/roles/list.html")]
    struct UsersTemplate {
        user: User,
        roles: Vec<UserRoleWithCount>,
    }

    let roles = query_as!(UserRoleWithCount, "select r.*, (select count(*) from users u where u.role_id = r.id) as 'members_count' from user_roles r")
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
    color: String,
}

#[derive(Template)]
#[template(path = "pages/admin/roles/new_edit.html")]
struct NewRoleTemplate {
    user: User,
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
        "insert into user_roles (name, reservations, guest_reservations) values ($1, $2, $3)",
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
    let color = CssColor::from_str(role.color.as_str()).unwrap_or(CssColor::None);
    let color = color.as_ref();
    query!(
        "update user_roles set name = $2, reservations = $3, guest_reservations = $4, color = $5 where id = $1",
        role_id,
        role.name,
        role.reservations,
        role.as_guest,
        color
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/roles")
        .body("Rolul a fost actualizat cu succes".to_string())
        .expect("Failed to create headers")
}

async fn delete_role(State(state): State<AppState>, Path(role_id): Path<i64>) -> impl IntoResponse {
    let users_with_role = query!(
        "select count(*) as 'count!' from users where role_id = $1",
        role_id
    )
    .fetch_one(&state.write_pool)
    .await
    .expect("Database error")
    .count;

    if users_with_role > 0 {
        return error_bubble_response(format!(
            "{users_with_role} utilizatori au acest rol, rolul nu poate fi șters"
        ));
    }

    query!("delete from user_roles where id = $1", role_id,)
        .execute(&state.write_pool)
        .await
        .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/roles")
        .body("Rolul a fost actualizat cu succes".to_string())
        .expect("Failed to create headers")
        .into_response()
}
