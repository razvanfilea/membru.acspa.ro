use std::collections::HashSet;

use argon2::password_hash::{Salt, SaltString};
use argon2::{password_hash, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use axum_login::{AuthnBackend, AuthzBackend, UserId};
use rand::{RngCore, SeedableRng};
use rand_hc::Hc128Rng;
use sqlx::{query_as, SqlitePool};
use tokio::task;

use crate::model::user::{User, UserCredentials};

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
    type User = User;
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
        query_as!(
            User,
            r#"select * from users_with_role where email = $1"#,
            user_id
        )
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
    let mut rng = Hc128Rng::from_os_rng();
    // let salt = SaltString::generate(rng); TODO use when updating argon
    let salt = {
        let mut bytes = [0u8; Salt::RECOMMENDED_LENGTH];
        rng.fill_bytes(&mut bytes);
        SaltString::encode_b64(&bytes).unwrap()
    };

    Argon2::default()
        .hash_password(password.as_ref().as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

pub fn validate_credentials<T: AsRef<str>, E: AsRef<str>>(
    password: T,
    expected_password_hash: E,
) -> Result<bool, password_hash::Error> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.as_ref())?;

    Ok(Argon2::default()
        .verify_password(password.as_ref().as_bytes(), &expected_password_hash)
        .is_ok())
}
