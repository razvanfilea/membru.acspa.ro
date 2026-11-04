mod breaks;
pub mod debtors;
mod payments;
pub mod payments_summary;

use crate::http::AppState;
use crate::http::auth::{PASSWORD_TOO_SHORT, generate_hash_from_password, is_valid_new_password};
use crate::http::error::{HttpError, HttpResult, OrBail, bail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::members::breaks::{add_break, delete_break};
use crate::http::pages::admin::members::payments::{add_payment, delete_payment};
use crate::http::pages::admin::members::payments_summary::{
    StatusGridTemplate, payments_status_partial,
};
use crate::http::response::{hx_notify_with, hx_redirect, hx_refresh};
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::{PaymentBreak, PaymentWithAllocations};
use crate::model::role::UserRole;
use crate::model::user::{User, UserDetails};
use crate::model::user_reservation::GroupedUserReservations;
use crate::reservation::remove_upcoming_reservations;
use crate::utils::date_formats::DateFormatExt;
use crate::utils::dates::{MonthIter, YearMonth};
use crate::utils::{date_formats, local_date, local_time};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use email_address::EmailAddress;
use serde::Deserialize;
use sqlx::{SqlitePool, query, query_as, query_scalar};
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
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
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
        "select * from user_details_with_role where name like $1 or email like $1 or role like $1 or nickname like $1 or phone_number like $1 or emergency_contact_phone_number like $1
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

#[derive(Template)]
#[template(path = "admin/members/new_edit_page.html")]
struct MemberFormTemplate {
    user: User,
    current_date: String,
    roles: Vec<String>,
    member_id: Option<i64>,
    values: MemberFormValues,
}

#[derive(Default)]
struct MemberFormValues {
    email: String,
    name: String,
    nickname: String,
    phone_number: String,
    emergency_contact_phone_number: String,
    role: String,
    birthday: String,
    member_since: String,
    received_gift: String,
    has_key: bool,
}

impl From<UserDetails> for MemberFormValues {
    fn from(user: UserDetails) -> Self {
        Self {
            email: user.email,
            name: user.name,
            nickname: user.nickname.unwrap_or_default(),
            phone_number: user.phone_number.unwrap_or_default(),
            emergency_contact_phone_number: user.emergency_contact_phone_number.unwrap_or_default(),
            role: user.role,
            birthday: user.birthday.to_iso(),
            member_since: user.member_since.to_iso(),
            received_gift: user
                .received_gift
                .map(|date| date.to_iso())
                .unwrap_or_default(),
            has_key: user.has_key,
        }
    }
}

#[derive(Deserialize)]
struct MemberForm {
    email: String,
    name: String,
    nickname: Option<String>,
    phone_number: Option<String>,
    emergency_contact_phone_number: Option<String>,
    role: String,
    has_key: Option<String>,
    birthday: String,
    member_since: String,
    received_gift: Option<String>,
    /// Only sent when creating a member
    password: Option<String>,
}

struct ValidatedMember {
    email: String,
    name: String,
    nickname: Option<String>,
    phone_number: Option<String>,
    emergency_contact_phone_number: Option<String>,
    role_id: i64,
    has_key: bool,
    birthday: Date,
    member_since: Date,
    received_gift: Option<Date>,
}

impl MemberForm {
    async fn validate(
        self,
        pool: &SqlitePool,
        member_id: Option<i64>,
    ) -> Result<ValidatedMember, HttpError> {
        fn parse_date(date: &str) -> Option<Date> {
            Date::parse(date, date_formats::ISO_DATE).ok()
        }

        fn non_empty(value: Option<String>) -> Option<String> {
            value
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        }

        let birthday = parse_date(&self.birthday).or_bail("Data nașterii este invalidă")?;
        let member_since =
            parse_date(&self.member_since).or_bail("Data înscrierii este invalidă")?;
        let received_gift = self.received_gift.as_deref().and_then(parse_date);

        let today = local_date();
        let email = self.email.trim().to_string();
        let name = self.name.trim().to_string();

        if name.is_empty() {
            return Err(bail("Numele este obligatoriu"));
        }

        if !EmailAddress::is_valid(&email) {
            return Err(bail("Adresa de email este invalidă"));
        }

        if User::email_exists_for_other(pool, &email, member_id).await? {
            return Err(bail("Adresa de email este deja folosită de alt utilizator"));
        }

        if birthday > today {
            return Err(bail("Data nașterii nu poate fi în viitor"));
        }
        if birthday.year() < 1900 {
            return Err(bail("Data nașterii este prea veche"));
        }

        if member_since > today {
            return Err(bail("Data înscrierii nu poate fi în viitor"));
        }

        // Payments and breaks before the join month would no longer be shown
        if let Some(member_id) = member_id {
            let first_covered = query_scalar!(
                r#"select min(month_start) as "month_start: Date" from (
                       select printf('%04d-%02d-01', a.year, a.month) as month_start
                       from payment_allocations a
                       inner join payments p on p.id = a.payment_id
                       where p.user_id = $1
                       union all
                       select start_date from payment_breaks where user_id = $1
                   )"#,
                member_id
            )
            .fetch_one(pool)
            .await?;

            if let Some(first_covered) = first_covered
                && YearMonth::from(first_covered) < YearMonth::from(member_since)
            {
                return Err(bail(format!(
                    "Există plăți sau pauze din {}, data înscrierii nu poate fi după această lună",
                    first_covered.to_month_year()
                )));
            }
        }

        if let Some(gift_date) = received_gift
            && gift_date < member_since
        {
            return Err(bail(
                "Data primirii cadoului nu poate fi înainte de înscriere",
            ));
        }

        let role_id = UserRole::fetch_id_by_name(pool, self.role.as_str())
            .await?
            .or_bail("Rolul selectat nu există")?;

        Ok(ValidatedMember {
            email,
            name,
            nickname: non_empty(self.nickname),
            phone_number: non_empty(self.phone_number),
            emergency_contact_phone_number: non_empty(self.emergency_contact_phone_number),
            role_id,
            has_key: self.has_key.is_some(),
            birthday,
            member_since,
            received_gift,
        })
    }
}

async fn new_member_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    let current_date = local_date().to_iso();

    MemberFormTemplate {
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
        roles: UserRole::fetch_all_names(&state.read_pool).await?,
        member_id: None,
        values: MemberFormValues {
            member_since: current_date.clone(),
            ..Default::default()
        },
        current_date,
    }
    .try_into_response()
}

async fn create_new_member(
    State(state): State<AppState>,
    Form(form): Form<MemberForm>,
) -> HttpResult {
    let password = form
        .password
        .clone()
        .filter(|password| is_valid_new_password(password))
        .or_bail(PASSWORD_TOO_SHORT)?;
    let member = form.validate(&state.read_pool, None).await?;
    let password_hash = generate_hash_from_password(password);

    let new_member_id = query_scalar!(
        "insert into users (email, name, nickname, phone_number, emergency_contact_phone_number, role_id, has_key, birthday, member_since, received_gift, password_hash)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) returning id as \"id!\"",
        member.email,
        member.name,
        member.nickname,
        member.phone_number,
        member.emergency_contact_phone_number,
        member.role_id,
        member.has_key,
        member.birthday,
        member.member_since,
        member.received_gift,
        password_hash,
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
        breaks: Vec<PaymentBreak>,
        status_grid: StatusGridTemplate,
    }

    let current_date = local_date();
    let current_year = current_date.year();
    let member = UserDetails::fetch(&state.read_pool, member_id).await?;
    let payments = PaymentWithAllocations::fetch_for_user(&state.read_pool, member_id).await?;
    let breaks = PaymentBreak::fetch_for_user(&state.read_pool, member_id).await?;
    let status_grid =
        StatusGridTemplate::new(member.clone(), current_year, true, payments, &breaks);

    ViewMemberTemplate {
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
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
        breaks,
        status_grid,
    }
    .try_into_response()
}

async fn edit_member_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(member_id): Path<i64>,
) -> HttpResult {
    MemberFormTemplate {
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
        current_date: local_date().to_iso(),
        roles: UserRole::fetch_all_names(&state.read_pool).await?,
        member_id: Some(member_id),
        values: UserDetails::fetch(&state.read_pool, member_id)
            .await?
            .into(),
    }
    .try_into_response()
}

async fn update_member(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    Form(form): Form<MemberForm>,
) -> HttpResult {
    let member = form.validate(&state.read_pool, Some(member_id)).await?;

    let updated = query!(
        "update users set email = $2, name = $3, role_id = $4, has_key = $5, birthday = $6, member_since = $7, received_gift = $8, nickname = $9, phone_number = $10, emergency_contact_phone_number = $11
         where id = $1 and is_deleted = false",
        member_id,
        member.email,
        member.name,
        member.role_id,
        member.has_key,
        member.birthday,
        member.member_since,
        member.received_gift,
        member.nickname,
        member.phone_number,
        member.emergency_contact_phone_number
    )
        .execute(&state.write_pool)
        .await?;

    if updated.rows_affected() == 0 {
        return Err(bail("Membrul nu există"));
    }

    Ok(hx_redirect(format!("/admin/members/view/{member_id}")))
}

async fn toggle_active_user(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/active_toggle.html")]
    struct ActiveToggleTemplate {
        member_id: i64,
        is_active: bool,
    }

    let is_active = query_scalar!(
        "update users set is_active = not is_active where id = $1 and is_deleted = false returning is_active",
        member_id
    )
    .fetch_optional(&state.write_pool)
    .await?
    .or_bail("Membrul nu există")?;

    let message = if is_active {
        "Membrul este activ"
    } else {
        "Membrul este inactiv"
    };
    hx_notify_with(
        ActiveToggleTemplate {
            member_id,
            is_active,
        }
        .render()?,
        message,
    )
}

async fn delete_member(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
) -> HttpResult {
    let user = auth_session.user().await.ok_or(HttpError::Unauthorized)?;
    if member_id == user.id {
        return Err(bail("Nu îți poți șterge propriul cont"));
    }

    let mut tx = state.write_pool.begin().await?;

    let cancelled =
        remove_upcoming_reservations(&mut tx, &state.location, local_time(), member_id, user.id)
            .await?;

    // Returning early drops the transaction, rolling back the removed reservations
    let deleted = query!(
        "update users set is_deleted = true where id = $1 and is_deleted = false",
        member_id
    )
    .execute(tx.as_mut())
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(bail("Membrul nu există"));
    }

    tx.commit().await?;

    if cancelled > 0 {
        let _ = state.reservation_notifier.send(());
    }

    Ok(hx_redirect("/admin/members"))
}

async fn clear_gift_dates(State(state): State<AppState>) -> HttpResult {
    query!("update users set received_gift = NULL where received_gift IS NOT NULL")
        .execute(&state.write_pool)
        .await?;

    Ok(hx_refresh())
}

#[derive(Deserialize)]
struct ChangePasswordForm {
    password: String,
}

async fn update_member_password(
    State(state): State<AppState>,
    Path(member_id): Path<i64>,
    Form(passwords): Form<ChangePasswordForm>,
) -> HttpResult {
    if !is_valid_new_password(&passwords.password) {
        return Err(bail(PASSWORD_TOO_SHORT));
    }
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

async fn member_reservations_year(
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
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/members/name_suggestions.html")]
    struct NameSuggestionsTemplate {
        names: Vec<String>,
    }

    let names =
        suggested_guest_names(&state.read_pool, state.location.id, query.name.trim()).await?;

    NameSuggestionsTemplate { names }.try_into_response()
}

/// The most recent past guests come first, followed by members. An empty name matches everyone.
async fn suggested_guest_names(
    pool: &SqlitePool,
    location: i64,
    name: &str,
) -> sqlx::Result<Vec<String>> {
    let pattern = format!("%{name}%");
    query_scalar!(
        r#"select name as "name!: String" from (
            select created_for as name, created_at as used_at from reservations
            where created_for like $1 and location = $2
            union all
            select coalesce(nickname, name), null from users
            where (name like $1 or nickname like $1) and is_deleted = false
        )
        group by name
        order by max(used_at) desc
        limit 3"#,
        pattern,
        location
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn guest_name_suggestions(pool: SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            "insert into users (id, email, name, password_hash, role_id, birthday, member_since) values
             (10, 'dan@acspa.ro', 'Dan Membru', '', 1, '1995-05-10', '2024-01-01'),
             (11, 'old@acspa.ro', 'Dan Sters', '', 1, '1995-05-10', '2024-01-01')",
        )
        .execute(&pool)
        .await?;
        sqlx::query("update users set is_deleted = true where id = 11")
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into reservations (user_id, location, date, hour, created_for, created_at) values
             (0, 1, '2026-01-01', 18, 'Ana Veche', '2026-01-01 10:00:00'),
             (0, 1, '2026-01-02', 18, 'Dana Noua', '2026-01-03 10:00:00'),
             (0, 1, '2026-01-03', 18, 'Dana Noua', '2026-01-02 10:00:00'),
             (0, 1, '2026-01-04', 18, 'Bogdan', '2026-01-02 12:00:00'),
             (0, 1, '2026-01-05', 18, 'Dan Membru', '2025-01-01 10:00:00')",
        )
        .execute(&pool)
        .await?;

        assert_eq!(
            suggested_guest_names(&pool, 1, "").await?,
            ["Dana Noua", "Bogdan", "Ana Veche"]
        );
        // A member who was also a guest shows once, deleted members not at all
        assert_eq!(
            suggested_guest_names(&pool, 1, "dan").await?,
            ["Dana Noua", "Bogdan", "Dan Membru"]
        );
        assert!(suggested_guest_names(&pool, 1, "zzz").await?.is_empty());
        // Guests of another location are not suggested
        assert!(suggested_guest_names(&pool, 2, "dana").await?.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn user_phone_numbers_crud_and_anonymization(pool: SqlitePool) -> sqlx::Result<()> {
        let user_id = query_scalar!(
            r#"
            insert into users (email, name, role_id, password_hash, birthday, member_since, phone_number, emergency_contact_phone_number)
            values ('member@acspa.ro', 'Test Member', 1, '', '1995-05-10', '2024-01-01', '+40712345678', '+40798765432')
            returning id as "id!"
            "#
        )
        .fetch_one(&pool)
        .await?;

        let user_details = UserDetails::fetch(&pool, user_id).await?;
        assert_eq!(user_details.phone_number.as_deref(), Some("+40712345678"));
        assert_eq!(
            user_details.emergency_contact_phone_number.as_deref(),
            Some("+40798765432")
        );

        query!(
            "update users set emergency_contact_phone_number = NULL where id = $1",
            user_id
        )
        .execute(&pool)
        .await?;

        let user_details = UserDetails::fetch(&pool, user_id).await?;
        assert_eq!(user_details.phone_number.as_deref(), Some("+40712345678"));
        assert_eq!(user_details.emergency_contact_phone_number, None);

        query!("update users set is_deleted = TRUE where id = $1", user_id)
            .execute(&pool)
            .await?;

        let raw_user = query!(
            "select email, name, phone_number, emergency_contact_phone_number from users where id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(
            raw_user.email,
            format!("deleted_{}@archived.acspa.ro", user_id)
        );
        assert_eq!(raw_user.phone_number, None);
        assert_eq!(raw_user.emergency_contact_phone_number, None);

        Ok(())
    }
}
