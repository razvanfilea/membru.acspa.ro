use chrono::{NaiveDate, NaiveDateTime};

pub struct FreeDay {
    pub date: NaiveDate,
    pub description: Option<String>,
    pub created_at: NaiveDateTime
}