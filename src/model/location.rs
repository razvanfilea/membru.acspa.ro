use std::borrow::Cow;

#[derive(Clone)]
pub struct Location {
    pub id: i64,
    pub name: Cow<'static, str>,
    pub slot_capacity: i64,
    pub slots_start_hour: i64,
    pub slot_duration: i64,
    pub slots_per_day: i64,
    pub alt_slots_start_hour: Option<i64>,
    pub alt_slot_duration: Option<i64>,
    pub alt_slots_per_day: Option<i64>,
}

impl Location {
    pub fn get_hour_structure(&self) -> HourStructure {
        HourStructure {
            slots_start_hour: self.slots_start_hour,
            slot_duration: self.slot_duration,
            slots_per_day: self.slots_per_day,
        }
    }

    pub fn get_alt_hour_structure(&self) -> HourStructure {
        HourStructure {
            slots_start_hour: self.alt_slots_start_hour.unwrap_or(self.slots_start_hour),
            slot_duration: self.alt_slot_duration.unwrap_or(self.slot_duration),
            slots_per_day: self.alt_slots_per_day.unwrap_or(self.slots_per_day),
        }
    }
}

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
