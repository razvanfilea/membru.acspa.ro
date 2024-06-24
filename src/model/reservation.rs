pub struct Reservation {
    pub user_id: i64,
    pub date: chrono::NaiveDate,
    pub hour: i64,
    pub location: i64,
    pub cancelled: bool,
    pub created_at: chrono::NaiveDateTime,
}
