use time::{Date, Duration, Weekday};

#[derive(Copy, Clone)]
pub struct DateIter {
    pub from: Date,
    pub to: Date,
    pub increment: Duration,
}

impl Iterator for DateIter {
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from > self.to {
            return None;
        }

        let next = self.from;
        self.from = self.from.saturating_add(self.increment);
        Some(next)
    }
}

impl DateIter {
    pub fn weeks_in_range(start_date: Date, end_date: Date) -> Self {
        assert!(start_date <= end_date);

        DateIter {
            from: start_date.prev_occurrence(Weekday::Monday),
            to: end_date.next_occurrence(Weekday::Sunday),
            increment: Duration::days(1),
        }
    }
}
