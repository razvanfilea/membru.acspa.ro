use time::{Date, Duration, Month, OffsetDateTime, Weekday};

pub fn local_time() -> OffsetDateTime {
    OffsetDateTime::now_local().expect("Failed to determine local offset")
}

pub fn local_date() -> Date {
    local_time().date()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct YearMonth {
    pub year: i32,
    pub month: Month,
}

impl YearMonth {
    pub fn new(year: i32, month: Month) -> Self {
        Self { year, month }
    }

    pub fn to_date(self) -> Date {
        Date::from_calendar_date(self.year, self.month, 1)
            .expect("The first of the month is always valid")
    }

    pub fn prev(self) -> Self {
        if self.month == Month::January {
            Self::new(self.year - 1, Month::December)
        } else {
            Self::new(self.year, self.month.previous())
        }
    }
}

impl From<Date> for YearMonth {
    fn from(date: Date) -> Self {
        Self::new(date.year(), date.month())
    }
}

#[derive(Clone)]
pub struct DateRangeIter {
    from: Date,
    to: Date,
}

impl Iterator for DateRangeIter {
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from > self.to {
            return None;
        }

        let next = self.from;
        self.from = self.from.saturating_add(Duration::days(1));
        Some(next)
    }
}

impl DateRangeIter {
    pub fn new(from: Date, to: Date) -> Self {
        Self { from, to }
    }

    pub fn weeks_in_range(start_date: Date, end_date: Date) -> Self {
        assert!(start_date <= end_date);

        Self::new(
            start_date.prev_occurrence(Weekday::Monday),
            end_date.next_occurrence(Weekday::Sunday),
        )
    }
}

#[derive(Clone)]
pub struct MonthIter {
    current: u8,
}

impl Default for MonthIter {
    fn default() -> Self {
        Self {
            current: Month::January as u8,
        }
    }
}

impl Iterator for MonthIter {
    type Item = Month;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > Month::December as u8 {
            return None;
        }

        let ret = Month::try_from(self.current).ok();
        self.current += 1;
        ret
    }
}
#[derive(Clone)]
pub struct YearMonthIter {
    current: YearMonth,
    end: YearMonth,
}

impl YearMonthIter {
    pub fn new(start: YearMonth, end_inclusive: YearMonth) -> Self {
        Self {
            current: start,
            end: end_inclusive,
        }
    }

    pub fn for_year(year: i32) -> Self {
        Self {
            current: YearMonth::new(year, Month::January),
            end: YearMonth::new(year, Month::December),
        }
    }
}

impl Iterator for YearMonthIter {
    type Item = YearMonth;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }

        let ret = self.current;

        // Advance to next month
        if self.current.month == Month::December {
            self.current.month = Month::January;
            self.current.year += 1;
        } else {
            self.current.month = self.current.month.next();
        }

        Some(ret)
    }
}
