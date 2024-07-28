use time::{Date, OffsetDateTime};

pub struct Reservation {
    pub user_id: i64,
    pub date: Date,
    pub hour: i64,
    pub location: i64,
    pub cancelled: bool,
    pub in_waiting: bool,
    pub created_at: OffsetDateTime,
}
