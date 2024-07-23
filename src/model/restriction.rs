use chrono::{NaiveDate, NaiveDateTime};

pub struct Restriction {
    pub date: NaiveDate,
    pub hour: Option<i64>,
    pub location: i64,
    pub message: String,
    pub created_at: NaiveDateTime
}
