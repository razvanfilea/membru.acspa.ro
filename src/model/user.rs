use axum_login::AuthUser;
use serde::Deserialize;
use time::Date;
use validator::Validate;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    #[allow(dead_code)]
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

#[derive(Clone, Deserialize, Validate)]
pub struct UserCredentials {
    #[validate(email(message = "Email invalid"))]
    pub email: String,
    #[validate(length(min = 8, message = "Parola este prea scurta"))]
    pub password: String,
}
