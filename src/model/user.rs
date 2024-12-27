use axum_login::AuthUser;
use serde::Deserialize;
use time::Date;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role_id: i64,
    pub role: String,
    pub has_key: bool,
    pub admin_panel_access: bool,
    pub birthday: Option<Date>,
    pub member_since: Option<Date>,
    pub received_gift: Option<Date>,
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

#[derive(Clone, Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}
