pub mod breaks;
pub mod debtors;
pub mod payments;
pub mod payments_summary;

use crate::http::AppState;
use crate::http::auth::generate_hash_from_password;
use crate::http::error::{HttpError, HttpResult, OrBail, bail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::breaks::{add_break, delete_break};
use crate::http::pages::admin::members::payments::{add_payment, delete_payment};
use crate::http::pages::admin::members::payments_summary::{
    MonthStatus, MonthStatusView, calculate_total_paid_for_year, calculate_year_status,
    payments_status_partial,
};
use crate::http::response::{hx_redirect, hx_refresh};
use axum::response::IntoResponse;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::role::UserRole;
use crate::model::user::{UserDetails, User};
use crate::model::user_reservation::GroupedUserReservations;
use crate::utils::date_formats::DateFormatExt;
use crate::utils::dates::{MonthIter, YearMonth, YearMonthIter};
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{query, query_as, query_scalar};
use std::collections::HashSet;
use time::Date;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(members_page))
        .route("/search_names", get(search_names))
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
        .route("/{id}/reservations/{year}", get(member_reservations_year))
        .route("/gifts", delete(clear_gift_dates))
}

async fn members_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/list_page.html")]
    struct MembersTemplate {
        user: User,
        members: Vec<UserDetails>,
    }

    let members = UserDetails::fetch_all(&state.read_pool).await?;

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
        members: Vec<UserDetails>,
    }

    let query = format!("%{}%", search_query.search);
    let sort_order = search_query.sort.to_sql_index();

    let members = query_as!(
        UserDetails,
        "select * from user_details_with_role where name like $1 or email like $1 or role like $1 or nickname like $1
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
        roles: UserRole::fetch_all_names(&state.read_pool).await?,
    }
    .try_into_response()
}

async fn create_new_member(
    State(state): State<AppState>,
    Form(new_member): Form<NewMember>,
) -> HttpResult {
    let role_id = UserRole::fetch_id_by_name(&state.read_pool, new_member.role.as_str())
        .await?
        .or_bail("Rolul selectat nu există")?;

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

    Ok(hx_redirect(format!("/admin/members/view/{new_member_id}")))
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
        member: UserDetails,
        current_date: Date,
        reservations: Vec<GroupedUserReservations>,
        allow_reservation_cancellation: bool,
        payments: Vec<PaymentWithAllocations>,
        breaks: Vec<PaymentBreak>,
        months_status_view: Vec<MonthStatusView>,
        total_paid: i64,
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
    let current_year = current_date.year();
    let member = UserDetails::fetch(&state.read_pool, member_id).await?;
    let payments = PaymentWithAllocations::fetch_for_user(&state.read_pool, member_id)
        .await
        .unwrap_or_default();

    let breaks = PaymentBreak::fetch_for_user(&state.read_pool, member_id).await?;
    let months_status_view = calculate_year_status(current_year, &member, &payments, &breaks);

    let total_paid = calculate_total_paid_for_year(&payments, current_year);

    ViewMemberTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        reservations: GroupedUserReservations::fetch_for_user_year(
            &state.read_pool,
            member.id,
            false,
            current_year,
        )
        .await?,
        current_date,
        member,
        allow_reservation_cancellation: false,
        payments,
        breaks,
        months_status_view,
        total_paid,
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
        existing_user: UserDetails,
    }

    EditMemberTemplate {
        current_date: local_date().to_iso(),
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        roles: UserRole::fetch_all_names(&state.read_pool).await?,
        existing_user: UserDetails::fetch(&state.read_pool, member_id).await?,
    }
    .try_into_response()
}

#[derive(Deserialize, Debug)]
struct UpdatedUser {
    email: String,
    name: String,
    nickname: Option<String>,
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

    // Parse dates
    let birthday =
        parse_date(Some(updated_user.birthday)).or_bail("Data nașterii este invalidă")?;
    let member_since =
        parse_date(Some(updated_user.member_since)).or_bail("Data înscrierii este invalidă")?;
    let received_gift = parse_date(updated_user.received_gift);

    let today = local_date();

    // Email uniqueness check
    if User::email_exists_for_other(&state.read_pool, &updated_user.email, member_id).await? {
        return Err(bail("Adresa de email este deja folosită de alt utilizator"));
    }

    // Birthday validation
    if birthday > today {
        return Err(bail("Data nașterii nu poate fi în viitor"));
    }
    if birthday.year() < 1900 {
        return Err(bail("Data nașterii este prea veche"));
    }

    // member_since validation
    if member_since > today {
        return Err(bail("Data înscrierii nu poate fi în viitor"));
    }

    // received_gift validation
    if let Some(gift_date) = received_gift {
        if gift_date < member_since {
            return Err(bail(
                "Data primirii cadoului nu poate fi înainte de înscriere",
            ));
        }
    }

    let role_id = UserRole::fetch_id_by_name(&state.read_pool, updated_user.role.as_str())
        .await?
        .or_bail("Rolul selectat nu există")?;
    let user_name = updated_user.name.trim();
    let nickname = updated_user.nickname.filter(|n| !n.trim().is_empty());
    let is_active = updated_user.is_active.is_some();
    let has_key = updated_user.has_key.is_some();

    query!(
        "update users set email = $2, name = $3, role_id = $4, has_key = $5, birthday = $6, member_since = $7, received_gift = $8, is_active = $9, nickname = $10
         where id = $1",
        member_id,
        updated_user.email,
        user_name,
        role_id,
        has_key,
        birthday,
        member_since,
        received_gift,
        is_active,
        nickname
    )
        .execute(&state.write_pool)
        .await?;

    Ok(hx_redirect(format!("/admin/members/view/{member_id}")))
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

    Ok(hx_refresh())
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
        existing_user: UserDetails,
    }

    ChangePasswordTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        existing_user: UserDetails::fetch(&state.read_pool, member_id).await?,
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

    Ok(hx_redirect("/admin/members"))
}

async fn clear_gift_dates(State(state): State<AppState>) -> HttpResult {
    query!("update users set received_gift = NULL where received_gift IS NOT NULL")
        .execute(&state.write_pool)
        .await?;

    Ok(hx_refresh())
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
    let user = UserDetails::fetch(&state.read_pool, member_id).await?;

    let new_password_hash = generate_hash_from_password(passwords.password);
    query!(
        "update users set password_hash = $1 where id = $2",
        new_password_hash,
        user.id
    )
    .execute(&state.write_pool)
    .await?;

    Ok(hx_redirect(format!("/admin/members/view/{member_id}")))
}

pub async fn member_reservations_year(
    State(state): State<AppState>,
    Path((member_id, year)): Path<(i64, i32)>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "components/reservations_with_year_selector.html")]
    struct ReservationsTemplate {
        member: UserDetails,
        reservations: Vec<GroupedUserReservations>,
        allow_reservation_cancellation: bool,
        current_year: i32,
        selected_year: i32,
        show_cancelled: bool,
        admin_reservations_view: bool,
    }

    let member = UserDetails::fetch(&state.read_pool, member_id).await?;
    let current_year = local_date().year();

    ReservationsTemplate {
        reservations: GroupedUserReservations::fetch_for_user_year(
            &state.read_pool,
            member_id,
            false,
            year,
        )
        .await?,
        member,
        allow_reservation_cancellation: false,
        current_year,
        selected_year: year,
        show_cancelled: false,
        admin_reservations_view: true,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct NameSearchQuery {
    name: String,
}

async fn search_names(
    State(state): State<AppState>,
    Query(query): Query<NameSearchQuery>,
) -> impl IntoResponse {
    let name = format!("%{}%", query.name.trim());
    if query.name.trim().is_empty() {
        return "".into_response();
    }

    let names = query_scalar!(
        "select coalesce(nickname, name) as 'name!' from users 
         where (name like $1 or nickname like $1) and is_deleted = false 
         limit 5",
        name
    )
    .fetch_all(&state.read_pool)
    .await;

    let Ok(names) = names else {
        return "".into_response();
    };

    if names.is_empty() {
        return "".into_response();
    }

    let html = names
        .into_iter()
        .map(|n| {
            format!(
                r#"<div class="px-4 py-2 hover:bg-primary hover:text-primary-content cursor-pointer border-b border-base-300 last:border-0" 
                        onclick="const input = this.closest('.relative').querySelector('input'); input.value = '{}'; this.closest('.search-results-container').innerHTML = '';">{}</div>"#,
                n, n
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<div class="absolute z-[60] bg-base-100 shadow-2xl rounded-xl border border-base-300 w-full mt-1 overflow-hidden">{}</div>"#, html).into_response()
}
