use chrono::Datelike;

pub type MonthDates = Vec<[chrono::NaiveDate; 7]>;

#[allow(deprecated)]
pub fn get_weeks_of_month(date: chrono::NaiveDate) -> MonthDates {
    let current_year = date.year();
    let current_month = date.month();

    // Get the first and last days of the current month
    let first_day_of_month = chrono::NaiveDate::from_ymd(current_year, current_month, 1);
    let last_day_of_month = if current_month == 12 {
        chrono::NaiveDate::from_ymd(current_year + 1, 1, 1) - chrono::Duration::days(1)
    } else {
        chrono::NaiveDate::from_ymd(current_year, current_month + 1, 1) - chrono::Duration::days(1)
    };

    let mut weeks = Vec::new();
    weeks.reserve_exact(6);

    {
        let first_week_day_index = first_day_of_month.weekday().num_days_from_monday() as i64;
        let mut current_date = first_day_of_month
            - chrono::Duration::days(if first_week_day_index == 0 {
                7
            } else {
                first_week_day_index
            });
        let mut week = [chrono::NaiveDate::default(); 7];
        let mut week_index = 0;

        // Iterate over each day in the month
        while current_date <= last_day_of_month {
            week[week_index] = current_date;
            week_index += 1;

            if current_date.weekday() == chrono::Weekday::Sun {
                weeks.push(week);
                week_index = 0;
            }

            current_date = current_date.succ_opt().unwrap();
        }

        // Add remaining days of the last week
        let last_week = weeks.last_mut().unwrap();
        while last_week.len() < 7 {
            last_week[week_index] = last_week.last().unwrap().succ();
            week_index += 1;
        }
    }

    // Ensure there are 6 weeks
    while weeks.len() < 6 {
        let mut week_index = 0;
        let mut next_week = [chrono::NaiveDate::default(); 7];
        let start_date = weeks.last().unwrap().last().unwrap().succ();
        for i in 0..7 {
            next_week[week_index] = start_date + chrono::Duration::days(i);
            week_index += 1;
        }
        weeks.push(next_week);
    }

    weeks
}
