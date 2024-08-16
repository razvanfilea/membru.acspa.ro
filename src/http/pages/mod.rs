use crate::http::auth::UserAuthenticator;
use crate::http::AppState;
use axum::routing::{get, post};
use axum::Router;
use axum_login::{login_required, permission_required};
use sqlx::{query_as, SqlitePool};
use crate::http::pages::user::login;
use crate::model::global_vars::GlobalVars;
use crate::model::user::UserUi;

mod admin;
mod home;
mod user;

pub type AuthSession = axum_login::AuthSession<UserAuthenticator>;

async fn get_global_vars(state: &AppState) -> GlobalVars {
    query_as!(GlobalVars, "select in_maintenance, entrance_code, homepage_message from global_vars")
        .fetch_one(&state.pool)
        .await
        .expect("Database error")
}

async fn get_user(pool: &SqlitePool, id: i64) -> UserUi {
    query_as!(UserUi, "select * from users_with_role where id = $1", id)
        .fetch_one(pool)
        .await
        .expect("Database error")

}

pub fn router() -> Router<AppState> {
    let admin_router =
        Router::new()
            .nest("/admin", admin::router())
            .route_layer(permission_required!(
                UserAuthenticator,
                login_url = "/",
                "admin_panel"
            ));

    let authenticated_router = Router::<AppState>::new()
        .merge(home::router())
        .merge(user::user_router())
        .route("/logout", post(login::logout))
        .route_layer(login_required!(UserAuthenticator, login_url = "/login"));

    let unauthenticated_router = Router::<AppState>::new()
        .route("/login", get(login::login_page))
        .route("/login", post(login::login))
        .route("/forgot_password", get(user::forgot_password));

    Router::new()
        .merge(admin_router)
        .merge(authenticated_router)
        .merge(unauthenticated_router)
}
