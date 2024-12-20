#[derive(Clone, PartialEq)]
pub struct HourStructure {
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
    pub description: Option<String>,
}

impl HourStructure {
    pub const fn new(
        slots_start_hour: i64,
        slot_duration: i64,
        slots_per_day: i64,
    ) -> HourStructure {
        HourStructure {
            slots_start_hour,
            slot_duration,
            slots_per_day,
            description: None,
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

pub const HOLIDAY_HOUR_STRUCTURE: HourStructure = HourStructure::new(10, 3, 4);
