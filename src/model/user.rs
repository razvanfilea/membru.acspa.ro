use axum_login::AuthUser;
use serde::Deserialize;
use sqlx::{SqliteExecutor, query_as, query_scalar};
use time::Date;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub nickname: Option<String>,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub admin_panel_access: bool,
}

#[derive(Debug, Clone)]
pub struct UserDetails {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub nickname: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub has_key: bool,
    pub admin_panel_access: bool,
    pub member_since: Date,
    pub birthday: Date,
    pub received_gift: Option<Date>,
    pub monthly_fee: Option<i64>,
}

impl Default for UserDetails {
    fn default() -> Self {
        Self {
            id: 0,
            email: String::new(),
            name: String::new(),
            nickname: None,
            role: String::new(),
            is_active: false,
            has_key: false,
            admin_panel_access: false,
            member_since: Date::MIN,
            birthday: Date::MIN,
            received_gift: None,
            monthly_fee: None,
        }
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 0,
            email: String::new(),
            name: String::new(),
            nickname: None,
            password_hash: String::new(),
            role: String::new(),
            is_active: false,
            admin_panel_access: false,
        }
    }
}

impl AuthUser for User {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.email.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

impl User {
    pub fn display_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.name)
    }

    pub async fn fetch_by_email(
        executor: impl SqliteExecutor<'_>,
        email: &str,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            "select * from users_with_role where email = $1",
            email
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn email_exists_for_other(
        executor: impl SqliteExecutor<'_>,
        email: &str,
        exclude_user_id: i64,
    ) -> sqlx::Result<bool> {
        let count = query_scalar!(
            "select count(*) from users where email = $1 and id != $2 and is_deleted = false",
            email,
            exclude_user_id
        )
        .fetch_one(executor)
        .await?;
        Ok(count > 0)
    }
}

impl UserDetails {
    pub fn display_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.name)
    }

    pub async fn fetch(executor: impl SqliteExecutor<'_>, id: i64) -> sqlx::Result<Self> {
        query_as!(
            Self,
            "select * from user_details_with_role where id = $1",
            id
        )
        .fetch_one(executor)
        .await
    }

    pub async fn fetch_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(Self, "select * from user_details_with_role order by name")
            .fetch_all(executor)
            .await
    }
}

#[derive(Clone, Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}
