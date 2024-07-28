use time::{Date, Duration, Weekday};

struct DateIter {
    pub from: Date,
    pub to: Date,
    pub increment: Duration,
}

impl Iterator for DateIter {
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from >= self.to {
            return None;
        }

        let next = self.from;
        self.from = self.from.saturating_add(self.increment);
        Some(next)
    }
}

pub type Weeks = Vec<[Date; 7]>;

pub fn get_weeks_in_range(start_date: Date, end_date: Date) -> Weeks {
    assert!(start_date <= end_date);

    DateIter {
        from: start_date.prev_occurrence(Weekday::Monday),
        to: end_date.next_occurrence(Weekday::Sunday),
        increment: Duration::weeks(1),
    }
    .map(|start_of_week| {
        [
            start_of_week,
            start_of_week.saturating_add(Duration::days(1)),
            start_of_week.saturating_add(Duration::days(2)),
            start_of_week.saturating_add(Duration::days(3)),
            start_of_week.saturating_add(Duration::days(4)),
            start_of_week.saturating_add(Duration::days(5)),
            start_of_week.saturating_add(Duration::days(6)),
        ]
    })
    .collect()
}
