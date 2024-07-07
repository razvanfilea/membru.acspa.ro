use crate::http::auth::generate_hash_from_password;
use crate::http::pages::AuthSession;
use crate::http::AppState;
use crate::model::global_vars::GlobalVars;
use crate::model::user::{BasicUser, UserDb};
use askama::Template;
use askama_axum::Response;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, Row};
use tracing::error;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/apply_settings", post(apply_settings))
        .route("/members", get(members_page))
        .route("/members/new", get(new_member_page))
        .route("/members/new", post(create_new_user))
}

async fn get_global_vars(state: &AppState) -> GlobalVars {
    query_as!(GlobalVars, "select * from global_vars")
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
}

async fn admin_page(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "pages/admin/index.html")]
    struct HomeTemplate {
        user: BasicUser,
        global_vars: GlobalVars,
    }

    HomeTemplate {
        user: auth_session.user.unwrap().into(),
        global_vars: get_global_vars(&state).await,
    }
}

#[derive(Deserialize)]
struct NewSettings {
    in_maintenance: Option<String>,
    entrance_code: String,
    reminder_message: String,
}

async fn apply_settings(
    State(state): State<AppState>,
    Form(settings): Form<NewSettings>,
) -> impl IntoResponse {
    let in_maintenance = settings.in_maintenance.is_some();
    query!(
        "update global_vars SET in_maintenance = $1, entrance_code = $2, reminder_message = $3",
        in_maintenance,
        settings.entrance_code,
        settings.reminder_message
    )
    .execute(&state.pool)
    .await
    .expect("Database error");

    "Setările au fost aplicate"
}

async fn members_page(
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
    #[template(path = "pages/admin/members_new.html")]
    struct NewMemberTemplate {
        user: BasicUser,
        roles: Vec<String>
    }

    let roles = query!("select role from user_roles")
        .fetch_all(&state.pool)
        .await
        .expect("Database error")
        .into_iter()
        .map(|record| record.role)
        .collect();

    NewMemberTemplate {
        user: auth_session.user.unwrap().into(),
        roles
    }
}

async fn create_new_user(
    State(state): State<AppState>,
    Form(new_user): Form<NewUser>,
) -> impl IntoResponse {
    struct ValidUser {
        valid: Option<i32>,
    }

    let role_is_valid = query_as!(
        ValidUser,
        "select exists(select 1 from user_roles where role = $1) as valid",
        new_user.role
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
