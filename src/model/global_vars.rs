use sqlx::{SqliteExecutor, query_as};

pub struct GlobalVars {
    pub in_maintenance: bool,
    pub check_payments: bool,
    pub entrance_code: String,
    pub homepage_message: String,
}

impl GlobalVars {
    pub async fn fetch(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Self> {
        query_as!(
            Self,
            "select in_maintenance, check_payments, entrance_code, homepage_message from global_vars"
        )
        .fetch_one(executor)
        .await
    }
}
