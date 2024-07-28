use time::{Date, OffsetDateTime};

pub struct FreeDay {
    pub date: Date,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}
