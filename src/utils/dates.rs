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

#[derive(Clone, Copy)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn year_month_new_creates_correct_struct() {
        let ym = YearMonth::new(2024, Month::March);
        assert_eq!(ym.year, 2024);
        assert_eq!(ym.month, Month::March);
    }

    #[test]
    fn year_month_prev_wraps_january_to_december() {
        let jan = YearMonth::new(2024, Month::January);
        let prev = jan.prev();
        assert_eq!(prev.year, 2023);
        assert_eq!(prev.month, Month::December);
    }

    #[test]
    fn year_month_prev_normal_month() {
        let march = YearMonth::new(2024, Month::March);
        let prev = march.prev();
        assert_eq!(prev.year, 2024);
        assert_eq!(prev.month, Month::February);
    }

    #[test]
    fn year_month_to_date_returns_first_of_month() {
        let ym = YearMonth::new(2024, Month::July);
        let d = ym.to_date();
        assert_eq!(d, date!(2024 - 07 - 01));
    }

    #[test]
    fn year_month_from_date() {
        let d = date!(2024 - 05 - 15);
        let ym = YearMonth::from(d);
        assert_eq!(ym.year, 2024);
        assert_eq!(ym.month, Month::May);
    }

    #[test]
    fn year_month_ordering() {
        let a = YearMonth::new(2023, Month::December);
        let b = YearMonth::new(2024, Month::January);
        let c = YearMonth::new(2024, Month::February);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
    }

    #[test]
    fn year_month_iter_for_year_iterates_all_12_months() {
        let months: Vec<_> = YearMonthIter::for_year(2024).collect();
        assert_eq!(months.len(), 12);
        assert_eq!(months[0], YearMonth::new(2024, Month::January));
        assert_eq!(months[11], YearMonth::new(2024, Month::December));
    }

    #[test]
    fn year_month_iter_custom_range() {
        let start = YearMonth::new(2024, Month::March);
        let end = YearMonth::new(2024, Month::June);
        let months: Vec<_> = YearMonthIter::new(start, end).collect();
        assert_eq!(months.len(), 4);
        assert_eq!(months[0].month, Month::March);
        assert_eq!(months[3].month, Month::June);
    }

    #[test]
    fn year_month_iter_across_year_boundary() {
        let start = YearMonth::new(2024, Month::November);
        let end = YearMonth::new(2025, Month::February);
        let months: Vec<_> = YearMonthIter::new(start, end).collect();
        assert_eq!(months.len(), 4);
        assert_eq!(months[0], YearMonth::new(2024, Month::November));
        assert_eq!(months[1], YearMonth::new(2024, Month::December));
        assert_eq!(months[2], YearMonth::new(2025, Month::January));
        assert_eq!(months[3], YearMonth::new(2025, Month::February));
    }

    #[test]
    fn year_month_iter_empty_when_start_after_end() {
        let start = YearMonth::new(2024, Month::June);
        let end = YearMonth::new(2024, Month::March);
        let months: Vec<_> = YearMonthIter::new(start, end).collect();
        assert!(months.is_empty());
    }

    #[test]
    fn month_iter_iterates_all_12_months() {
        let months: Vec<_> = MonthIter::default().collect();
        assert_eq!(months.len(), 12);
        assert_eq!(months[0], Month::January);
        assert_eq!(months[11], Month::December);
    }

    #[test]
    fn date_range_iter_basic() {
        let from = date!(2024 - 07 - 10);
        let to = date!(2024 - 07 - 13);
        let dates: Vec<_> = DateRangeIter::new(from, to).collect();
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[0], date!(2024 - 07 - 10));
        assert_eq!(dates[3], date!(2024 - 07 - 13));
    }

    #[test]
    fn date_range_iter_single_day() {
        let d = date!(2024 - 07 - 15);
        let dates: Vec<_> = DateRangeIter::new(d, d).collect();
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0], d);
    }

    #[test]
    fn date_range_iter_empty_when_from_after_to() {
        let from = date!(2024 - 07 - 15);
        let to = date!(2024 - 07 - 10);
        let dates: Vec<_> = DateRangeIter::new(from, to).collect();
        assert!(dates.is_empty());
    }

    #[test]
    fn date_range_iter_weeks_in_range_expands_to_week_boundaries() {
        // Wednesday to Friday
        let start = date!(2024 - 07 - 10); // Wednesday
        let end = date!(2024 - 07 - 12); // Friday
        let iter = DateRangeIter::weeks_in_range(start, end);

        // Should expand to Monday-Sunday
        let dates: Vec<_> = iter.collect();
        assert_eq!(dates[0], date!(2024 - 07 - 08)); // Previous Monday
        assert_eq!(dates[dates.len() - 1], date!(2024 - 07 - 14)); // Next Sunday
        assert_eq!(dates.len(), 7);
    }
}
