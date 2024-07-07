use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, State};
use axum::{Form, Router};
use axum::routing::{get, post};
use serde::Deserialize;
use sqlx::{query, query_as};
use tracing::error;
use crate::http::AppState;
use crate::http::auth::generate_hash_from_password;
use crate::http::pages::admin::{admin_page, apply_settings};
use crate::http::pages::AuthSession;
use crate::model::user::{BasicUser, UserDb};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(members_page))
        .route("/new", get(new_member_page))
        .route("/new", post(create_new_user))
        .route("/edit/:id", get(edit_member_page))
        .route("/edit/:id", post(update_user))
}

async fn get_all_roles(state: &AppState) -> Vec<String> {
    query!("select role from user_roles")
        .fetch_all(&state.pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|record| record.role)
        .collect()
}

async fn is_role_valid(state: &AppState, role: impl AsRef<str>) -> bool {
    struct ValidRole {
        valid: Option<i32>,
    }

    let role = role.as_ref();
    let role_is_valid = query_as!(
        ValidRole,
        "select exists(select 1 from user_roles where role = $1) as valid",
        role
    )
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
        .valid
        .is_some();

    // TODO Error handling

    if role_is_valid {
        error!("Role is invalid!!!");
    }
    
    role_is_valid
}

pub async fn members_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members.html")]
    struct MembersTemplate {
        user: BasicUser,
        members: Vec<BasicUser>,
    }

    let members = query_as!(UserDb, "select * from users")
        .fetch_all(&state.pool)
        .await
        .unwrap()
        .into_iter()
        .map(BasicUser::from)
        .collect();

    MembersTemplate {
        user: auth_session.user.unwrap().into(),
        members,
    }
}

#[derive(Deserialize)]
pub struct NewUser {
    email: String,
    name: String,
    role: String,
    has_key: Option<String>,
    password: String,
}

pub async fn new_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members_new.html")]
    struct NewMemberTemplate {
        user: BasicUser,
        roles: Vec<String>
    }

    NewMemberTemplate {
        user: auth_session.user.unwrap().into(),
        roles: get_all_roles(&state).await
    }
}

pub async fn create_new_user(
    State(state): State<AppState>,
    Form(new_user): Form<NewUser>,
) -> impl IntoResponse {
    let is_role_valid = is_role_valid(&state, new_user.role.as_str()).await;

    let has_key = new_user.has_key.is_some();
    let password_hash = generate_hash_from_password(new_user.password);
    query!(
        "insert into users (email, name, role, has_key, password_hash) VALUES ($1, $2, $3, $4, $5)",
        new_user.email,
        new_user.name,
        new_user.role,
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


pub async fn edit_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/members_edit.html")]
    struct NewMemberTemplate {
        user: BasicUser,
        roles: Vec<String>,
        existing_user: BasicUser,
    }

    let existing_user = query_as!(UserDb, "select * from users where id = $1", user_id)
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
        .into();

    NewMemberTemplate {
        user: auth_session.user.unwrap().into(),
        roles: get_all_roles(&state).await,
        existing_user
    }
}

#[derive(Deserialize)]
pub struct ExistingUser {
    name: String,
    role: String,
    has_key: Option<String>,
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Form(user): Form<ExistingUser>,
) -> impl IntoResponse {
    let is_role_valid = is_role_valid(&state, user.role.as_str()).await;

    let has_key = user.has_key.is_some();
    query!(
        "update users set name = $2, role = $3, has_key = $4 where id = $1",
        user_id,
        user.name,
        user.role,
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
