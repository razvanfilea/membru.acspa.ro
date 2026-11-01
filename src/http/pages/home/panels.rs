use crate::http::AppState;
use crate::http::pages::admin::schedule_overrides::calendar::{DayEvents, get_day_markers};
use crate::http::pages::home::DAYS_AHEAD_ALLOWED;
use crate::http::pages::home::socket::HoursTemplate;
use crate::model::user::User;
use crate::utils::date_formats::{self, DateFormatExt};
use crate::utils::dates::DateRangeIter;
use askama::Template;
use std::collections::HashMap;
use time::Date;

/// The selectable date, today or up to `DAYS_AHEAD_ALLOWED` days ahead
pub fn clamp_date(date: Date, today: Date) -> Date {
    if date >= today && date <= today + DAYS_AHEAD_ALLOWED {
        date
    } else {
        today
    }
}

fn window_title(from: Date, to: Date) -> String {
    let month = |date: Date| date_formats::month_as_str(&date.month());

    if from.month() == to.month() {
        format!("{} {}", month(from), from.year())
    } else if from.year() == to.year() {
        format!("{} – {} {}", month(from), month(to), to.year())
    } else {
        format!(
            "{} {} – {} {}",
            month(from),
            from.year(),
            month(to),
            to.year()
        )
    }
}

#[derive(Template)]
#[template(path = "home/calendar.html")]
pub struct CalendarTemplate {
    title: String,
    current_date: Date,
    selected_date: Date,
    last_day: Date,
    days: DateRangeIter,
    markers: HashMap<Date, DayEvents>,
}

impl CalendarTemplate {
    pub async fn render(
        state: &AppState,
        today: Date,
        selected_date: Date,
    ) -> sqlx::Result<String> {
        let last_day = today + DAYS_AHEAD_ALLOWED;

        Ok(Self {
            title: window_title(today, last_day),
            current_date: today,
            selected_date,
            last_day,
            days: DateRangeIter::weeks_in_range(today, last_day),
            markers: get_day_markers(state, today, last_day).await?,
        }
        .to_string())
    }
}

#[derive(Template)]
#[template(path = "home/day_panel.html")]
pub struct DayPanelTemplate {
    selected_date: Date,
    /// Only set when the user is allowed to make reservations
    hours_html: Option<String>,
    has_paid: bool,
}

impl DayPanelTemplate {
    /// `None` when the hours could not be loaded
    pub async fn render(
        state: &AppState,
        selected_date: Date,
        user: &User,
        has_paid: bool,
    ) -> Option<String> {
        let hours_html = if user.is_active && has_paid {
            Some(HoursTemplate::create_response(state, selected_date, user, true).await?)
        } else {
            None
        };

        Some(
            Self {
                selected_date,
                hours_html,
                has_paid,
            }
            .to_string(),
        )
    }
}
