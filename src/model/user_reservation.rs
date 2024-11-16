use time::{Date, OffsetDateTime};
use crate::utils::local_time;

pub struct UserReservation {
    pub date: Date,
    pub hour: i64,

    pub as_guest: bool,

    pub cancelled: bool,
    pub in_waiting: bool,

    pub created_at: OffsetDateTime,
}

impl UserReservation {
    pub fn is_cancellable(&self) -> bool {
        let now = local_time();
        let now_date = now.date();
        !self.cancelled
            && (self.date > now_date
            || (self.date == now_date && self.hour as u8 >= now.time().hour()))
    }
}

