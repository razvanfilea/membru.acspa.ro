use std::borrow::Cow;

pub struct Location {
    pub id: i64,
    pub name: Cow<'static, str>,
    pub slot_capacity: i64,
    pub waiting_capacity: i64,
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
}

impl Location {
    pub fn get_hour_structure(&self) -> HourStructure {
        HourStructure {
            slots_start_hour: self.slots_start_hour,
            slot_duration: self.slot_duration,
            slots_per_day: self.slots_per_day,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct HourStructure {
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
}

impl HourStructure {
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.slots_per_day)
            .map(|step| (self.slots_start_hour + self.slot_duration * step) as u8)
    }

    pub fn is_hour_valid(&self, hour: u8) -> bool {
        self.iter().any(|valid_hour| valid_hour == hour)
    }
}
