use axum_login::AuthUser;
use serde::Deserialize;
use time::Date;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role_id: i64,
    pub role: String,
    pub is_active: bool,
    pub has_key: bool,
    pub admin_panel_access: bool,
    pub member_since: Date,
    pub birthday: Date,
    pub received_gift: Option<Date>,
    #[allow(dead_code)]
    pub is_deleted: bool,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 0,
            email: String::new(),
            name: String::new(),
            password_hash: String::new(),
            role_id: 0,
            role: String::new(),
            is_active: false,
            has_key: false,
            admin_panel_access: false,
            member_since: Date::MIN,
            birthday: Date::MIN,
            received_gift: None,
            is_deleted: false,
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

#[derive(Clone, Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}
