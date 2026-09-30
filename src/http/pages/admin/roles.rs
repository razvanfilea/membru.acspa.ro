use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail, bail};
use crate::http::pages::AuthSession;
use crate::http::response::hx_redirect;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::payment::format_amount;
use crate::model::role::UserRole;
use crate::model::user::User;
use crate::utils::CssColor;
use askama::Template;
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{query, query_as};
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
        pub monthly_fee: Option<i64>,
        pub members_count: i64,
    }

    impl UserRoleWithCount {
        fn monthly_fee_amount(&self) -> Option<String> {
            self.monthly_fee.map(format_amount)
        }

        fn css_color(&self) -> CssColor {
            CssColor::parse_or_none(self.color.as_deref())
        }
    }

    #[derive(Template)]
    #[template(path = "admin/roles/list_page.html")]
    struct UsersTemplate {
        user: User,
        roles: Vec<UserRoleWithCount>,
    }

    let roles = query_as!(UserRoleWithCount, "select r.*, (select count(*) from users u where u.role_id = r.id and u.is_deleted = false) as 'members_count' from user_roles r")
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
    monthly_fee: Option<String>,
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

struct ValidatedRole {
    name: String,
    reservations: i64,
    as_guest: i64,
    color: CssColor,
    /// In cents, `None` when the role has no fee
    monthly_fee: Option<i64>,
}

impl NewRole {
    fn validate(self) -> HttpResult<ValidatedRole> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(bail("Numele rolului este obligatoriu"));
        }
        if self.reservations < 0 || self.as_guest < 0 {
            return Err(bail("Numărul de rezervări nu poate fi negativ"));
        }

        let monthly_fee = match self.monthly_fee.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(fee) => {
                let fee = fee
                    .parse::<f64>()
                    .ok()
                    .filter(|fee| fee.is_finite() && *fee >= 0.0)
                    .or_bail("Cotizația lunară este invalidă")?;
                Some((fee * 100.0).round() as i64).filter(|cents| *cents != 0)
            }
        };

        Ok(ValidatedRole {
            name,
            reservations: self.reservations,
            as_guest: self.as_guest,
            color: CssColor::parse_or_none(Some(&self.color)),
            monthly_fee,
        })
    }
}

fn role_saved(result: sqlx::Result<SqliteQueryResult>) -> HttpResult<()> {
    match result {
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            Err(bail("Există deja un rol cu acest nume"))
        }
        result => result.map(|_| ()).map_err(Into::into),
    }
}

async fn create_new_role(State(state): State<AppState>, Form(role): Form<NewRole>) -> HttpResult {
    let role = role.validate()?;
    let color = role.color.as_ref();

    role_saved(
        query!(
            "insert into user_roles (name, reservations, guest_reservations, color, monthly_fee) values ($1, $2, $3, $4, $5)",
            role.name,
            role.reservations,
            role.as_guest,
            color,
            role.monthly_fee
        )
        .execute(&state.write_pool)
        .await,
    )?;

    Ok(hx_redirect("/admin/roles"))
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
        return Ok(hx_redirect("/admin/roles"));
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
    let role = role.validate()?;
    let color = role.color.as_ref();

    role_saved(
        query!(
            "update user_roles set name = $2, reservations = $3, guest_reservations = $4, color = $5, monthly_fee = $6 where id = $1",
            role_id,
            role.name,
            role.reservations,
            role.as_guest,
            color,
            role.monthly_fee
        )
        .execute(&state.write_pool)
        .await,
    )?;

    Ok(hx_redirect("/admin/roles"))
}

async fn delete_role(State(state): State<AppState>, Path(role_id): Path<i64>) -> HttpResult {
    // Deleted members keep their role, so they block the delete too
    let counts = query!(
        r#"select count(*) as "total!: i64", ifnull(sum(is_deleted), 0) as "deleted!: i64"
           from users where role_id = $1"#,
        role_id
    )
    .fetch_one(&state.write_pool)
    .await?;

    if counts.total > 0 {
        let message = if counts.deleted > 0 {
            format!(
                "{} utilizatori au acest rol (dintre care {} membri șterși), rolul nu poate fi șters",
                counts.total, counts.deleted
            )
        } else {
            format!(
                "{} utilizatori au acest rol, rolul nu poate fi șters",
                counts.total
            )
        };
        return Err(bail(message));
    }

    query!("delete from user_roles where id = $1", role_id,)
        .execute(&state.write_pool)
        .await?;

    Ok(hx_redirect("/admin/roles"))
}
