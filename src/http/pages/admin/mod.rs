use axum::Router;
use axum::routing::get;
use crate::http::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_page))
}

async fn admin_page() {
    
}