use crate::http::auth::UserAuthenticator;
use crate::http::AppState;
use axum::routing::{get, post};
use axum::Router;
use axum_login::{login_required, permission_required};
use user::{change_password, login, profile};

mod admin;
mod home;
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
        .route("/profile", get(profile::profile_page))
        .route("/profile/reservations", post(profile::profile_reservations))
        .route("/logout", post(login::logout))
        .nest("/change_password", change_password::router())
        .route_layer(login_required!(UserAuthenticator, login_url = "/login"));

    let unauthenticated_router = Router::<AppState>::new()
        .route("/login", get(login::login_page))
        .route("/login", post(login::login));

    Router::new()
        .merge(admin_router)
        .merge(authenticated_router)
        .merge(unauthenticated_router)
}
