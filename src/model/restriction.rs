use time::{Date, OffsetDateTime};

pub struct Restriction {
    pub date: Date,
    pub hour: Option<i64>,
    pub message: String,
    pub created_at: OffsetDateTime,
}
