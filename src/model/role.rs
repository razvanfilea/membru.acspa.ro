use sqlx::{SqliteExecutor, query_as, query_scalar};

pub struct UserRole {
    pub id: i64,
    pub name: String,
    pub reservations: i64,
    pub guest_reservations: i64,
    pub color: Option<String>,
    #[allow(dead_code)]
    pub admin_panel_access: bool,
}

impl UserRole {
    pub async fn fetch(executor: impl SqliteExecutor<'_>, id: i64) -> sqlx::Result<Self> {
        query_as!(Self, "select * from user_roles where id = $1", id)
            .fetch_one(executor)
            .await
    }

    pub async fn fetch_all_names(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<String>> {
        query_scalar!("select name from user_roles")
            .fetch_all(executor)
            .await
    }

    pub async fn fetch_id_by_name(
        executor: impl SqliteExecutor<'_>,
        name: &str,
    ) -> sqlx::Result<Option<i64>> {
        query_scalar!("select id from user_roles where name = $1", name)
            .fetch_optional(executor)
            .await
    }
}
