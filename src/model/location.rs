#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Location {
    pub id: i64,
    pub name: String,
    pub slot_capacity: i64,
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
    pub alt_slots_start_hour: Option<i64>,
    pub alt_slot_duration: Option<i64>,
    pub alt_slots_per_day: Option<i64>,
}
