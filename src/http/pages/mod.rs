use crate::http::auth::UserAuthenticator;
use crate::http::AppState;
use axum::routing::{get, post};
use axum::Router;
use axum_login::login_required;

mod home;
mod login;
mod profile;

pub type AuthSession = axum_login::AuthSession<UserAuthenticator>;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(home::router())
        .route("/profile", get(profile::profile_page))
        .route("/profile/reservations", post(profile::profile_reservations))
        .route("/logout", post(login::logout))
        .route_layer(login_required!(UserAuthenticator, login_url = "/login"))
        .route("/login", get(login::login_page))
        .route("/login", post(login::login))
}
