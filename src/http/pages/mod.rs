use crate::http::AppState;
use crate::http::auth::UserAuthenticator;
use crate::http::pages::user::login;
use axum::Router;
use axum::routing::{get, post};
use axum_login::{login_required, permission_required};

mod admin;
mod home;
pub mod notification_template;
mod user;

pub type AuthSession = axum_login::AuthSession<UserAuthenticator>;

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
