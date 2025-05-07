use crate::http::AppState;
use crate::http::auth::generate_hash_from_password;
use crate::http::error::HttpResult;
use crate::http::pages::{AuthSession, get_user};
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::utils::{date_formats, local_time};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as};
use time::Date;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(members_page))
        .route("/search", post(search_members))
        .route("/new", get(new_member_page))
        .route("/new", post(create_new_user))
        .route("/edit/{id}", get(edit_member_page))
        .route("/edit/{id}", post(update_user))
        .route("/change_password/{id}", get(change_password_page))
        .route("/change_password/{id}", post(update_password))
        .route("/delete/{id}", post(delete_user))
}

async fn get_all_roles(state: &AppState) -> Vec<String> {
    query!("select name from user_roles")
        .map(|record| record.name)
        .fetch_all(&state.read_pool)
        .await
        .expect("Database error")
}

async fn get_role_id(state: &AppState, role: impl AsRef<str>) -> Option<i64> {
    let role = role.as_ref();
    query!("select id from user_roles where name = $1", role)
        .fetch_optional(&state.read_pool)
        .await
        .expect("Database error")
        .map(|row| row.id)
}

fn map_date_to_string(date: &Option<Date>) -> String {
    date.map(|date| date.format(date_formats::READABLE_DATE).unwrap())
        .unwrap_or_else(|| "?".to_string())
}

async fn members_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/members/list_page.html")]
    struct MembersTemplate {
        user: User,
        members: Vec<User>,
    }

    let members = query_as!(User, "select * from users_with_role order by name")
        .fetch_all(&state.read_pool)
        .await
        .expect("Database error");

    MembersTemplate {
        user: auth_session.user.expect("User should be logged in"),
        members,
    }
    .into_response()
}

#[derive(Deserialize)]
enum MembersSortOrder {
    Alphabetical,
    Birthday,
    Gift,
}

#[derive(Deserialize)]
struct SearchQuery {
    search: String,
    sort: MembersSortOrder,
}

async fn search_members(
    State(state): State<AppState>,
    Form(search_query): Form<SearchQuery>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/list_content.html")]
    struct MembersListTemplate {
        members: Vec<User>,
    }

    let query = format!("%{}%", search_query.search);
    let sort_order = match search_query.sort {
        MembersSortOrder::Alphabetical => "name",
        MembersSortOrder::Birthday => "birthday",
        MembersSortOrder::Gift => "received_gift",
    };

    let members = query_as!(
        User,
        "select * from users_with_role where name like $1 or email like $1 or role like $1
         order by case 
          when $2 = 'name' then name
          when $2 = 'birthday' then birthday
          when $2 = 'received_gift' then received_gift
         end, email, role",
        query,
        sort_order
    )
    .fetch_all(&state.read_pool)
    .await?;

    MembersListTemplate { members }.try_into_response()
}

#[derive(Deserialize)]
struct NewUser {
    email: String,
    name: String,
    role: String,
    password: String,
}

async fn new_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/members/new_page.html")]
    struct NewMemberTemplate {
        user: User,
        roles: Vec<String>,
    }

    NewMemberTemplate {
        user: auth_session.user.expect("User should be logged in"),
        roles: get_all_roles(&state).await,
    }
    .into_response()
}

async fn create_new_user(
    State(state): State<AppState>,
    Form(new_user): Form<NewUser>,
) -> impl IntoResponse {
    let role_id = get_role_id(&state, new_user.role.as_str())
        .await
        .expect("Invalid role");

    let user_name = new_user.name.trim();
    let password_hash = generate_hash_from_password(new_user.password);
    query!(
        "insert into users (email, name, role_id, password_hash, member_since) VALUES ($1, $2, $3, $4, date('now'))",
        new_user.email,
        user_name,
        role_id,
        password_hash
    )
        .execute(&state.write_pool)
        .await
        .expect("Database error");

    [("HX-Redirect", "/admin/members")]
}

async fn edit_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/members/edit_page.html")]
    struct EditMemberTemplate {
        current_date: String,
        user: User,
        roles: Vec<String>,
        existing_user: User,
    }

    EditMemberTemplate {
        current_date: local_time().date().format(date_formats::ISO_DATE).unwrap(),
        user: auth_session.user.expect("User should be logged in"),
        roles: get_all_roles(&state).await,
        existing_user: get_user(&state.read_pool, user_id).await,
    }
    .into_response()
}

#[derive(Deserialize, Debug)]
struct UpdatedUser {
    email: String,
    name: String,
    role: String,
    has_key: Option<String>,
    birthday: Option<String>,
    member_since: Option<String>,
    received_gift: Option<String>,
}

async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Form(updated_user): Form<UpdatedUser>,
) -> HttpResult {
    fn parse_date(date: Option<String>) -> Option<Date> {
        date.filter(|date| !date.is_empty() && date != "yyyy-mm-dd")
            .and_then(|date| Date::parse(date.as_str(), date_formats::ISO_DATE).ok())
    }

    let role_id = get_role_id(&state, updated_user.role.as_str())
        .await
        .expect("Invalid role");
    let user_name = updated_user.name.trim();
    let has_key = updated_user.has_key.is_some();
    let birthday = parse_date(updated_user.birthday);
    let member_since = parse_date(updated_user.member_since);
    let received_gift = parse_date(updated_user.received_gift);

    query!(
        "update users set email = $2, name = $3, role_id = $4, has_key = $5, birthday = $6, member_since = $7, received_gift = $8 where id = $1",
        user_id,
        updated_user.email,
        user_name,
        role_id,
        has_key,
        birthday,
        member_since,
        received_gift
    )
    .execute(&state.write_pool)
    .await?;

    Ok([("HX-Redirect", "/admin/members")].into_response())
}

async fn change_password_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "admin/members/change_password.html")]
    struct ChangePasswordTemplate {
        user: User,
        existing_user: User,
    }

    ChangePasswordTemplate {
        user: auth_session.user.expect("User should be logged in"),
        existing_user: get_user(&state.read_pool, user_id).await,
    }
    .into_response()
}

async fn delete_user(State(state): State<AppState>, Path(user_id): Path<i64>) -> HttpResult {
    query!("delete from reservations where user_id = $1", user_id)
        .execute(&state.write_pool)
        .await?;

    query!("delete from users where id = $1", user_id)
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Redirect", "/admin/members")].into_response())
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    password: String,
}

pub async fn update_password(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Form(passwords): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    let user = get_user(&state.read_pool, user_id).await;

    let new_password_hash = generate_hash_from_password(passwords.password);
    query!(
        "update users set password_hash = $1 where id = $2",
        new_password_hash,
        user.id
    )
    .execute(&state.write_pool)
    .await
    .expect("Database error");

    [("HX-Redirect", "/admin/")]
}
