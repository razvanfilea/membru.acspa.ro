use crate::model::user::{UserCredentials, UserDb};
use argon2::password_hash;
use axum::async_trait;
use axum_login::{AuthnBackend, AuthzBackend, UserId};
use sqlx::{query_as, SqlitePool};
use std::collections::HashSet;
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
    type User = UserDb;
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
        query_as!(UserDb, "select * from users where email = $1", user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(std::io::Error::other)
    }
}

#[async_trait]
impl AuthzBackend for UserAuthenticator {
    type Permission = ();

    async fn get_user_permissions(
        &self,
        _user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        todo!()
    }

    async fn get_group_permissions(
        &self,
        _user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        todo!()
    }

    async fn get_all_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        todo!()
    }

    async fn has_perm(
        &self,
        user: &Self::User,
        perm: Self::Permission,
    ) -> Result<bool, Self::Error> {
        todo!()
    }
}

pub fn validate_credentials<T: AsRef<str>, E: AsRef<str>>(
    password: T,
    expected_password_hash: E,
) -> Result<bool, password_hash::Error> {
    Ok(password.as_ref() == expected_password_hash.as_ref())
    // let expected_password_hash = PasswordHash::new(expected_password_hash.as_ref())?;

    // return Ok(Argon2::default()
    //     .verify_password(password.as_ref().as_bytes(), &expected_password_hash)
    //     .is_ok());
}
