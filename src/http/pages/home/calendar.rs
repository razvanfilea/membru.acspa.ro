use time::{Date, Duration, Month, Weekday};

pub type MonthDates = Vec<[Date; 7]>;

pub fn get_weeks_of_month(date: Date) -> MonthDates {
    let current_year = date.year();
    let current_month = date.month();

    // Get the first and last days of the current month
    let first_day_of_month = Date::from_calendar_date(current_year, current_month, 1).unwrap();
    let last_day_of_month = if current_month == Month::December {
        Date::from_calendar_date(current_year + 1, Month::January, 1).unwrap() - Duration::days(1)
    } else {
        Date::from_calendar_date(current_year, current_month.next(), 1).unwrap() - Duration::days(1)
    };

    let mut weeks = Vec::new();
    weeks.reserve_exact(6);

    {
        let first_week_day_index = first_day_of_month.weekday().number_from_monday() as i64;
        let mut current_date = first_day_of_month
            - Duration::days(if first_week_day_index == 0 {
                7
            } else {
                first_week_day_index
            });
        let mut week = [Date::MIN; 7];
        let mut week_index = 0;

        // Iterate over each day in the month
        while current_date <= last_day_of_month {
            week[week_index] = current_date;
            week_index += 1;

            if current_date.weekday() == Weekday::Sunday {
                weeks.push(week);
                week_index = 0;
            }

            current_date = current_date.next_day().unwrap();
        }

        // Add remaining days of the last week
        let last_week = weeks.last_mut().unwrap();
        while last_week.len() < 7 {
            last_week[week_index] = last_week.last().unwrap().next_day().unwrap();
            week_index += 1;
        }
    }

    // Ensure there are 6 weeks
    while weeks.len() < 6 {
        let mut week_index = 0;
        let mut next_week = [Date::MIN; 7];
        let start_date = weeks.last().unwrap().last().unwrap().next_day().unwrap();
        for i in 0..7 {
            next_week[week_index] = start_date + Duration::days(i);
            week_index += 1;
        }
        weeks.push(next_week);
    }

    weeks
}
