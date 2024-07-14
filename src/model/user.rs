use axum_login::AuthUser;
use serde::Deserialize;
use validator::Validate;

// #[derive(Debug, Clone, sqlx::FromRow)]
// pub struct UserDb {
//     pub id: i64,
//     pub email: String,
//     pub name: String,
//     pub password_hash: String,
//     pub role_id: i64,
//     pub has_key: bool,
// }

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserUi {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
    pub has_key: bool,
    pub admin_panel_access: bool,
}

impl AuthUser for UserUi {
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
