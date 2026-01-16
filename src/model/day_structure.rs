use crate::model::location::Location;
use sqlx::{SqliteExecutor, query_as};
use time::{Date, Weekday};

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

    /// Fetches the alternative day structure for a given date.
    /// Returns `Some` if there's an override or it's a weekend, `None` for regular weekdays.
    pub async fn fetch_for_date(
        executor: impl SqliteExecutor<'_>,
        date: Date,
    ) -> sqlx::Result<Option<Self>> {
        fn is_weekend(weekday: Weekday) -> bool {
            weekday == Weekday::Saturday || weekday == Weekday::Sunday
        }

        let day = query_as!(
            Self,
            "select slots_start_hour, slots_start_minute, slot_duration, slots_per_day, description, slot_capacity, consumes_reservation
             from alternative_days where date = $1",
            date
        ).fetch_optional(executor).await?;

        Ok(day.or_else(|| {
            if is_weekend(date.weekday()) {
                Some(HOLIDAY_DAY_STRUCTURE)
            } else {
                None
            }
        }))
    }

    /// Fetches the day structure for a date, falling back to the location's default.
    pub async fn fetch_or_default(
        executor: impl SqliteExecutor<'_>,
        date: Date,
        location: &Location,
    ) -> sqlx::Result<Self> {
        Ok(Self::fetch_for_date(executor, date)
            .await?
            .unwrap_or_else(|| location.day_structure()))
    }
}

pub const HOLIDAY_DAY_STRUCTURE: DayStructure = DayStructure::new(10, 3, 4, true);
