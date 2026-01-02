#[derive(Clone, PartialEq)]
pub struct DayStructure {
    pub slots_start_hour: i64,
    pub slots_start_minute: Option<i64>,
    pub slot_duration: i64,
    pub slots_per_day: i64,
    pub description: Option<String>,
    pub slot_capacity: Option<i64>,
    pub consumes_reservation: bool,
}

impl DayStructure {
    pub const fn new(
        slots_start_hour: i64,
        slot_duration: i64,
        slots_per_day: i64,
        consumes_reservation: bool,
    ) -> DayStructure {
        DayStructure {
            slots_start_hour,
            slots_start_minute: None,
            slot_duration,
            slots_per_day,
            description: None,
            slot_capacity: None,
            consumes_reservation,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.slots_per_day)
            .map(|step| (self.slots_start_hour + self.slot_duration * step) as u8)
    }

    pub fn is_hour_valid(&self, hour: u8) -> bool {
        self.iter().any(|valid_hour| valid_hour == hour)
    }
}

pub const HOLIDAY_DAY_STRUCTURE: DayStructure = DayStructure::new(10, 3, 4, true);
