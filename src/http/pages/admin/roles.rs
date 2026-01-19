use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, bail};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::utils::CssColor;
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
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
        .route("/edit/{id}", get(edit_role_page))
        .route("/edit/{id}", post(update_role))
        .route("/{id}", delete(delete_role))
}

async fn roles_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
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
    #[template(path = "admin/roles/list_page.html")]
    struct UsersTemplate {
        user: User,
        roles: Vec<UserRoleWithCount>,
    }

    let roles = query_as!(UserRoleWithCount, "select r.*, (select count(*) from users u where u.role_id = r.id) as 'members_count' from user_roles r")
        .fetch_all(&state.read_pool)
        .await?;

    UsersTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        roles,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct NewRole {
    name: String,
    reservations: i64,
    as_guest: i64,
    color: String,
}

#[derive(Template)]
#[template(path = "admin/roles/new_edit_page.html")]
struct NewOrEditRoleTemplate {
    user: User,
    current: Option<UserRole>,
}

async fn new_role_page(auth_session: AuthSession) -> HttpResult {
    NewOrEditRoleTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        current: None,
    }
    .try_into_response()
}

async fn create_new_role(State(state): State<AppState>, Form(role): Form<NewRole>) -> HttpResult {
    query!(
        "insert into user_roles (name, reservations, guest_reservations) values ($1, $2, $3)",
        role.name,
        role.reservations,
        role.as_guest
    )
    .execute(&state.write_pool)
    .await?;

    Ok([("HX-Redirect", "/admin/roles")].into_response())
}

async fn edit_role_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(role_id): Path<i64>,
) -> HttpResult {
    let role = query_as!(UserRole, "select * from user_roles where id = $1", role_id)
        .fetch_optional(&state.read_pool)
        .await?;

    if role.is_none() {
        return Ok([("HX-Redirect", "/admin/roles")].into_response());
    }

    NewOrEditRoleTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        current: role,
    }
    .try_into_response()
}

async fn update_role(
    State(state): State<AppState>,
    Path(role_id): Path<i64>,
    Form(role): Form<NewRole>,
) -> HttpResult {
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
    .await?;

    Ok([("HX-Redirect", "/admin/roles")].into_response())
}

async fn delete_role(State(state): State<AppState>, Path(role_id): Path<i64>) -> HttpResult {
    let users_with_role = query!(
        "select count(*) as 'count!' from users where role_id = $1",
        role_id
    )
    .fetch_one(&state.write_pool)
    .await?
    .count;

    if users_with_role > 0 {
        return Err(bail(format!(
            "{users_with_role} utilizatori au acest rol, rolul nu poate fi șters"
        )));
    }

    query!("delete from user_roles where id = $1", role_id,)
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Redirect", "/admin/roles")].into_response())
}
