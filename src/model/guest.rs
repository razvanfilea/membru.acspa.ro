use chrono::{NaiveDate, NaiveDateTime};

pub struct SpecialGuest {
    name: String,
    location: i64,
    date: NaiveDate,
    hour: i64,
    created_by: i64,
    created_at: NaiveDateTime,
}
