use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};
use tracing::error;

use crate::http::auth::generate_hash_from_password;
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::user::UserUi;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(members_page))
        .route("/new", get(new_member_page))
        .route("/new", post(create_new_user))
        .route("/edit/:id", get(edit_member_page))
        .route("/edit/:id", post(update_user))
}

async fn get_all_roles(state: &AppState) -> Vec<String> {
    query!("select name from user_roles")
        .fetch_all(&state.pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|record| record.name)
        .collect()
}

async fn get_role_id(state: &AppState, role: impl AsRef<str>) -> Option<i64> {
    struct RoleId {
        id: i64,
    }

    let role = role.as_ref();
    query_as!(RoleId, "select id from user_roles where name = $1", role)
        .fetch_optional(&state.pool)
        .await
        .expect("Database error")
        .map(|row| row.id)
}

async fn members_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members/list.html")]
    struct MembersTemplate {
        user: UserUi,
        members: Vec<UserUi>,
    }

    let members = query_as!(UserUi, "select * from users_with_role")
        .fetch_all(&state.pool)
        .await
        .expect("Database error");

    MembersTemplate {
        user: auth_session.user.unwrap(),
        members,
    }
}

#[derive(Deserialize)]
struct NewUser {
    email: String,
    name: String,
    role: String,
    has_key: Option<String>,
    password: String,
}

async fn new_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members/new.html")]
    struct NewMemberTemplate {
        user: UserUi,
        roles: Vec<String>,
    }

    NewMemberTemplate {
        user: auth_session.user.unwrap(),
        roles: get_all_roles(&state).await,
    }
}

async fn create_new_user(
    State(state): State<AppState>,
    Form(new_user): Form<NewUser>,
) -> impl IntoResponse {
    let role_id = get_role_id(&state, new_user.role.as_str())
        .await
        .expect("Invalid role");

    let has_key = new_user.has_key.is_some();
    let password_hash = generate_hash_from_password(new_user.password);
    query!(
        "insert into users (email, name, role_id, has_key, password_hash) VALUES ($1, $2, $3, $4, $5)",
        new_user.email,
        new_user.name,
        role_id,
        has_key,
        password_hash
    )
        .execute(&state.pool)
        .await
        .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/members")
        .body("Utilizatorul a fost creat cu success".to_string())
        .map_err(|e| {
            error!("Failed to return headers: {e}");
            "OOps".to_string() // TODO
        })
}

async fn edit_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members/edit.html")]
    struct NewMemberTemplate {
        user: UserUi,
        roles: Vec<String>,
        existing_user: UserUi,
    }

    let existing_user = query_as!(
        UserUi,
        "select * from users_with_role where id = $1",
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .expect("Database error");

    NewMemberTemplate {
        user: auth_session.user.unwrap(),
        roles: get_all_roles(&state).await,
        existing_user,
    }
}

#[derive(Deserialize)]
struct ExistingUser {
    name: String,
    role: String,
    has_key: Option<String>,
}

async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Form(user): Form<ExistingUser>,
) -> impl IntoResponse {
    let role_id = get_role_id(&state, user.role.as_str())
        .await
        .expect("Invalid role");

    let has_key = user.has_key.is_some();
    query!(
        "update users set name = $2, role_id = $3, has_key = $4 where id = $1",
        user_id,
        user.name,
        role_id,
        has_key
    )
    .execute(&state.pool)
    .await
    .expect("Database error");

    Response::builder()
        .header("HX-Redirect", "/admin/members")
        .body("Utilizatorul a fost creat cu success".to_string())
        .map_err(|e| {
            error!("Failed to return headers: {e}");
            "OOps".to_string() // TODO
        })
}
