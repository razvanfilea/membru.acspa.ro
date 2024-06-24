use axum_login::AuthUser;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UserDb {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
    pub has_key: bool,
}

impl AuthUser for UserDb {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.email.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

pub struct BasicUser {
    pub email: String,
    pub name: String,
    pub role: String,
    pub has_key: bool,
}

impl From<UserDb> for BasicUser {
    fn from(value: UserDb) -> Self {
        BasicUser {
            email: value.email,
            name: value.name,
            role: value.role,
            has_key: value.has_key,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct UserCredentials {
    #[validate(email(message = "Email invalid"))]
    pub email: String,
    #[validate(length(min = 8, message = "Parola este prea scurta"))]
    pub password: String,
}
