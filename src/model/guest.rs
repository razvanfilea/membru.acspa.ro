use time::{Date, OffsetDateTime};

pub struct SpecialGuest {
    name: String,
    location: i64,
    date: Date,
    hour: i64,
    created_by: i64,
    created_at: OffsetDateTime
}
