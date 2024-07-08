use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::role::UserRole;
use crate::model::user::{UserCredentials, UserDb, UserUi};
use argon2::password_hash::rand_core::SeedableRng;
use argon2::password_hash::SaltString;
use argon2::{password_hash, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::async_trait;
use axum_login::{AuthnBackend, AuthzBackend, UserId};
use rand_hc::Hc128Rng;
use sqlx::{query_as, SqlitePool};
use tokio::task;

#[derive(Clone)]
pub struct UserAuthenticator {
    pool: SqlitePool,
}

impl UserAuthenticator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuthnBackend for UserAuthenticator {
    type User = UserUi;
    type Credentials = UserCredentials;
    type Error = std::io::Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let Some(user) = self.get_user(&creds.email).await? else {
            return Ok(None);
        };

        task::spawn_blocking(|| {
            Ok(
                if validate_credentials(creds.password, &user.password_hash)
                    .map_err(std::io::Error::other)?
                {
                    Some(user)
                } else {
                    None
                },
            )
        })
        .await
        .expect("Password verification failed unexpectedly")
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        query_as!(UserUi, r#"select * from users_with_role where email = $1"#, user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(std::io::Error::other)
    }
}

#[async_trait]
impl AuthzBackend for UserAuthenticator {
    type Permission = String;

    async fn get_all_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        Ok(HashSet::from([
            user.role.clone(),
            if user.admin_panel_access {
                "admin_panel".to_string()
            } else {
                "".to_string()
            },
        ]))
    }
}

pub fn generate_hash_from_password<T: AsRef<str>>(password: T) -> String {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let rng = Hc128Rng::seed_from_u64(since_the_epoch.as_millis() as u64);
    let salt = SaltString::generate(rng);

    return Argon2::default()
        .hash_password(password.as_ref().as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
}

pub fn validate_credentials<T: AsRef<str>, E: AsRef<str>>(
    password: T,
    expected_password_hash: E,
) -> Result<bool, password_hash::Error> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.as_ref())?;

    return Ok(Argon2::default()
        .verify_password(password.as_ref().as_bytes(), &expected_password_hash)
        .is_ok());
}
