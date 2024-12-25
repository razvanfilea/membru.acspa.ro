use crate::model::day_structure::DayStructure;
use std::borrow::Cow;

pub struct Location {
    pub id: i64,
    pub name: Cow<'static, str>,
    pub slot_capacity: i64,
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
}

impl Location {
    pub fn day_structure(&self) -> DayStructure {
        DayStructure::new(
            self.slots_start_hour,
            self.slot_duration,
            self.slots_per_day,
        )
    }
}
