use crate::http::AppState;
use crate::model::user::UserUi;
use chrono::NaiveDate;

pub async fn create_reservation(
    state: &AppState,
    user: UserUi,
    selected_date: NaiveDate,
    selected_hour: u8,
) -> Result<String, String> {
    let transaction = state.pool.begin().await.expect("Database error");
    
    Ok("".to_string())
}
