mod breaks;
pub mod debtors;
mod payments;
mod payments_summary;

use crate::http::AppState;
use crate::http::auth::generate_hash_from_password;
use crate::http::error::{HttpError, HttpResult, OrBail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::breaks::{
    add_break, delete_break, get_user_payment_breaks,
};
use crate::http::pages::admin::members::payments::{
    add_payment, delete_payment, get_user_payments,
};
use crate::http::pages::admin::members::payments_summary::MonthStatus;
use crate::http::pages::admin::members::payments_summary::{
    MonthStatusView, calculate_year_status, payments_status_partial,
};
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::user::User;
use crate::utils::dates::YearMonthIter;
use crate::utils::dates::{MonthIter, YearMonth};
use crate::utils::queries::{GroupedUserReservations, get_user, get_user_reservations};
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, query_scalar};
use std::collections::HashSet;
use time::Date;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(members_page))
        .route("/search", post(search_members))
        .route("/new", get(new_member_page))
        .route("/new", post(create_new_member))
        .route("/view/{id}", get(view_member_page))
        .route("/edit/{id}", get(edit_member_page))
        .route("/edit/{id}", post(update_member))
        .route("/change_password/{id}", get(change_password_page))
        .route("/change_password/{id}", post(update_member_password))
        .route("/toggle_active/{id}", post(toggle_active_user))
        .route("/delete/{id}", post(delete_member))
        .route("/payments/{id}", post(add_payment))
        .route("/payments/{id}", delete(delete_payment))
        .route("/breaks/{id}", post(add_break))
        .route("/breaks/{id}", delete(delete_break))
        .route("/payment_status/{id}/{year}", get(payments_status_partial))
}

async fn get_all_roles(state: &AppState) -> sqlx::Result<Vec<String>> {
    query_scalar!("select name from user_roles")
        .fetch_all(&state.read_pool)
        .await
}

async fn get_role_id(state: &AppState, role: &str) -> sqlx::Result<Option<i64>> {
    query_scalar!("select id from user_roles where name = $1", role)
        .fetch_optional(&state.read_pool)
        .await
}

fn map_date_to_string(date: &Option<Date>) -> String {
    date.map(|date| date_formats::as_readable(&date))
        .unwrap_or_else(|| "?".to_string())
}

async fn members_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/list_page.html")]
    struct MembersTemplate {
        user: User,
        members: Vec<User>,
    }

    let members = query_as!(User, "select * from users_with_role order by name")
        .fetch_all(&state.read_pool)
        .await?;

    MembersTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        members,
    }
    .try_into_response()
}

#[derive(Clone, Copy, Deserialize)]
enum MembersSortOrder {
    Alphabetical,
    Birthday,
    Gift,
    ClosestBirthday,
}

impl MembersSortOrder {
    fn to_sql_index(self) -> u8 {
        match self {
            MembersSortOrder::Alphabetical => 0,
            MembersSortOrder::Birthday => 1,
            MembersSortOrder::Gift => 2,
            MembersSortOrder::ClosestBirthday => 3,
        }
    }
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
    #[template(path = "admin/members/list_page.html", block = "list")]
    struct MembersListTemplate {
        members: Vec<User>,
    }

    let query = format!("%{}%", search_query.search);
    let sort_order = search_query.sort.to_sql_index();

    let members = query_as!(
        User,
        "select * from users_with_role where name like $1 or email like $1 or role like $1
         order by case 
          when $2 = 0 then name
          when $2 = 1 then birthday
          when $2 = 2 then received_gift
          when $2 = 3 then ((strftime('%j', birthday) - strftime('%j', 'now') + 365) % 365)
         end, email, role",
        query,
        sort_order
    )
    .fetch_all(&state.read_pool)
    .await?;

    MembersListTemplate { members }.try_into_response()
}

#[derive(Deserialize)]
struct NewMember {
    email: String,
    name: String,
    role: String,
    birthday: Date,
    password: String,
}

async fn new_member_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/new_page.html")]
    struct NewMemberTemplate {
        user: User,
        roles: Vec<String>,
    }

    NewMemberTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        roles: get_all_roles(&state).await?,
    }
    .try_into_response()
}

async fn create_new_member(
    State(state): State<AppState>,
    Form(new_member): Form<NewMember>,
) -> HttpResult {
    let role_id = get_role_id(&state, new_member.role.as_str())
        .await?
        .expect("Invalid role");

    let user_name = new_member.name.trim();
    let password_hash = generate_hash_from_password(new_member.password);
    let new_member_id = query_scalar!(
        "insert into users (email, name, role_id, password_hash, birthday, member_since) values ($1, $2, $3, $4, $5, date('now')) returning id",
        new_member.email,
        user_name,
        role_id,
        password_hash,
        new_member.birthday,
    )
        .fetch_one(&state.write_pool)
        .await?;

    Ok([(
        "HX-Redirect",
        format!("/admin/members/view/{new_member_id}"),
    )]
    .into_response())
}

async fn view_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(member_id): Path<i64>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/view_page.html")]
    struct ViewMemberTemplate {
        user: User,
        member: User,
        current_date: Date,
        reservations: Vec<GroupedUserReservations>,
        allow_reservation_cancellation: bool,
        payments: Vec<PaymentWithAllocations>,
        breaks: Vec<PaymentBreak>,
        months_status_view: Vec<MonthStatusView>,
    }

    impl ViewMemberTemplate {
        pub fn get_paid_months_json(&self) -> String {
            let mut months: HashSet<String> = self
                .payments
                .iter()
                .flat_map(|p| p.allocations.iter())
                .map(|alloc| {
                    // Formats as M-YYYY
                    format!("{}-{:04}", alloc.month as u8, alloc.year)
                })
                .collect();

            for br in &self.breaks {
                let start = YearMonth::from(br.start_date);
                let end = YearMonth::from(br.end_date);

                for ym in YearMonthIter::new(start, end) {
                    months.insert(format!("{}-{:04}", ym.month as u8, ym.year));
                }
            }

            serde_json::to_string(&months).expect("Failed to serialize")
        }
    }

    let current_date = local_date();
    let member = get_user(&state.read_pool, member_id).await?;
    let payments = get_user_payments(&state.read_pool, member_id)
        .await
        .unwrap_or_default();

    let breaks = get_user_payment_breaks(&state.read_pool, member_id)
        .await
        .unwrap_or_default();
    let months_status_view =
        calculate_year_status(current_date.year(), &member, &payments, &breaks);

    ViewMemberTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        reservations: get_user_reservations(&state.read_pool, member.id, false).await,
        current_date,
        member,
        allow_reservation_cancellation: false,
        payments,
        breaks,
        months_status_view,
    }
    .try_into_response()
}

async fn edit_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(member_id): Path<i64>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/edit_page.html")]
    struct EditMemberTemplate {
        current_date: String,
        user: User,
        roles: Vec<String>,
        existing_user: User,
    }

    EditMemberTemplate {
        current_date: date_formats::as_iso(&local_date()),
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        roles: get_all_roles(&state).await?,
        existing_user: get_user(&state.read_pool, member_id).await?,
    }
    .try_into_response()
}

#[derive(Deserialize, Debug)]
struct UpdatedUser {
    email: String,
    name: String,
    role: String,
    is_active: Option<String>,
    has_key: Option<String>,
    birthday: String,
    member_since: String,
    received_gift: Option<String>,
}

async fn update_member(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    Form(updated_user): Form<UpdatedUser>,
) -> HttpResult {
    fn parse_date(date: Option<String>) -> Option<Date> {
        date.filter(|date| !date.is_empty() && date != "yyyy-mm-dd")
            .and_then(|date| Date::parse(date.as_str(), date_formats::ISO_DATE).ok())
    }

    let role_id = get_role_id(&state, updated_user.role.as_str())
        .await?
        .or_bail("Rolul selectat nu există")?;
    let user_name = updated_user.name.trim();
    let is_active = updated_user.is_active.is_some();
    let has_key = updated_user.has_key.is_some();
    let Some(birthday) = parse_date(Some(updated_user.birthday)) else {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    };
    let Some(member_since) = parse_date(Some(updated_user.member_since)) else {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    };
    let received_gift = parse_date(updated_user.received_gift);

    query!(
        "update users set email = $2, name = $3, role_id = $4, has_key = $5, birthday = $6, member_since = $7, received_gift = $8, is_active = $9
         where id = $1",
        member_id,
        updated_user.email,
        user_name,
        role_id,
        has_key,
        birthday,
        member_since,
        received_gift,
        is_active
    )
        .execute(&state.write_pool)
        .await?;

    Ok([("HX-Redirect", format!("/admin/members/view/{member_id}"))].into_response())
}

async fn toggle_active_user(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
) -> HttpResult {
    query!(
        "update users set is_active = not is_active where id = $1",
        member_id
    )
    .execute(&state.write_pool)
    .await?;

    Ok([("HX-Refresh", "true")].into_response())
}

async fn change_password_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(member_id): Path<i64>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/change_password.html")]
    struct ChangePasswordTemplate {
        user: User,
        existing_user: User,
    }

    ChangePasswordTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        existing_user: get_user(&state.read_pool, member_id).await?,
    }
    .try_into_response()
}

async fn delete_member(State(state): State<AppState>, Path(member_id): Path<i64>) -> HttpResult {
    let mut tx = state.write_pool.begin().await?;

    query!("delete from reservations where user_id = $1", member_id)
        .execute(tx.as_mut())
        .await?;

    query!(
        "update users set is_deleted = true where id = $1 ",
        member_id
    )
    .execute(tx.as_mut())
    .await?;

    tx.commit().await?;

    Ok([("HX-Redirect", "/admin/members")].into_response())
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    password: String,
}

pub async fn update_member_password(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    Form(passwords): Form<ChangePasswordForm>,
) -> HttpResult {
    let user = get_user(&state.read_pool, member_id).await?;

    let new_password_hash = generate_hash_from_password(passwords.password);
    query!(
        "update users set password_hash = $1 where id = $2",
        new_password_hash,
        user.id
    )
    .execute(&state.write_pool)
    .await?;

    Ok([("HX-Redirect", format!("/admin/members/view/{member_id}"))].into_response())
}
