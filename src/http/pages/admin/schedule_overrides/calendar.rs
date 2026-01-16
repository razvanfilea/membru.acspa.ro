use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::schedule_overrides::AlternativeDay;
use crate::http::pages::admin::schedule_overrides::holidays::{
    get_holiday, get_holidays_for_month,
};
use crate::http::pages::admin::schedule_overrides::restrictions::{
    get_restrictions_for_day, get_restrictions_for_month,
};
use crate::http::pages::admin::schedule_overrides::tournaments::{
    get_tournament_day, get_tournament_days,
};
use crate::http::pages::home::socket::HoursTemplate;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::day_structure::DayStructure;
use crate::model::restriction::Restriction;
use crate::model::user::User;
use crate::utils::date_formats::DateFormatExt;
use crate::utils::dates::DateRangeIter;
use crate::utils::{date_formats, local_date};
use askama::Template;
use axum::Router;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use std::collections::HashMap;
use time::{Date, Month};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(calendar_page_redirect)) // Redirect to current month
        .route("/{year}/{month}", get(calendar_page))
        .route("/details/{date}", get(day_details_partial))
}

async fn calendar_page_redirect() -> impl IntoResponse {
    let today = local_date();
    axum::response::Redirect::to(&format!(
        "/admin/calendar/{}/{}",
        today.year(),
        today.month() as u8
    ))
}

#[derive(Default, Clone, Copy)]
struct DayEvents {
    has_holiday: bool,
    has_tournament: bool,
    has_restriction: bool,
}

#[derive(Template)]
#[template(path = "admin/calendar/calendar_page.html")]
struct CalendarTemplate {
    user: User,
    current_date: Date,
    calendar_days: DateRangeIter,
    day_markers: HashMap<Date, DayEvents>, // Markers for the calendar grid
    selected_date: Date,
    selected_holiday: Option<AlternativeDay>,
    selected_tournament: Option<AlternativeDay>,
    selected_restrictions: Vec<Restriction>,
    day_structure: DayStructure,
    // Navigation
    prev_month: (i32, u8),
    next_month: (i32, u8),
    reservations: String,
}

impl CalendarTemplate {
    fn can_add_restriction(&self) -> bool {
        !self.selected_restrictions.iter().any(|r| r.hour.is_none())
    }

    fn is_restriction_hour_available(&self, hour: i64) -> bool {
        !self
            .selected_restrictions
            .iter()
            .any(|r| r.hour == Some(hour))
    }
}

async fn calendar_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path((year, month_u8)): Path<(i32, u8)>,
) -> HttpResult {
    let today = local_date();
    let month = Month::try_from(month_u8).unwrap_or(today.month());
    let selected_date = Date::from_calendar_date(year, month, 1).or_bail("Data este invalida")?;

    let holidays = get_holidays_for_month(&state.read_pool, selected_date).await?;
    let tournaments = get_tournament_days(&state.read_pool, Some(selected_date)).await?;
    let restrictions = get_restrictions_for_month(&state.read_pool, selected_date).await?;

    let mut day_markers: HashMap<Date, DayEvents> = HashMap::new();
    for h in &holidays {
        day_markers.entry(h.date).or_default().has_holiday = true;
    }
    for t in &tournaments {
        day_markers.entry(t.date).or_default().has_tournament = true;
    }
    for r in &restrictions {
        day_markers.entry(r.date).or_default().has_restriction = true;
    }

    // Calculate calendar range (full weeks)
    let last_day = if month == Month::December {
        Date::from_calendar_date(year + 1, Month::January, 1)
            .unwrap()
            .previous_day()
            .unwrap()
    } else {
        Date::from_calendar_date(year, month.next(), 1)
            .unwrap()
            .previous_day()
            .unwrap()
    };
    let calendar_days = DateRangeIter::weeks_in_range(selected_date, last_day);

    let mut fake_user = User::default();
    fake_user.admin_panel_access = true;
    let reservations =
        HoursTemplate::create_response(&state, selected_date, &fake_user, false).await;

    // Navigation logic
    let prev_month_date = selected_date.previous_day().unwrap();
    let next_month_date = last_day.next_day().unwrap();

    CalendarTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        current_date: today,
        calendar_days,
        day_markers,
        selected_date,
        selected_holiday: holidays.into_iter().find(|d| d.date == today),
        selected_tournament: tournaments.into_iter().find(|d| d.date == today),
        selected_restrictions: restrictions
            .into_iter()
            .filter(|r| r.date == today)
            .collect(),
        day_structure: DayStructure::fetch_or_default(&state.read_pool, today, &state.location)
            .await?,
        prev_month: (prev_month_date.year(), prev_month_date.month() as u8),
        next_month: (next_month_date.year(), next_month_date.month() as u8),
        reservations,
    }
    .try_into_response()
}

async fn day_details_partial(
    State(state): State<AppState>,
    Path(date_str): Path<String>,
) -> HttpResult {
    let date = Date::parse(&date_str, date_formats::ISO_DATE).or_bail("Data este invalida")?;

    day_details_response(state, date).await
}

#[derive(Template)]
#[template(path = "admin/calendar/day_details_response.html")]
struct DayDetailsTemplate {
    current_date: Date,
    selected_date: Date,
    selected_holiday: Option<AlternativeDay>,
    selected_tournament: Option<AlternativeDay>,
    selected_restrictions: Vec<Restriction>,
    day_structure: DayStructure,
    events: DayEvents,
    reservations: String,
}

impl DayDetailsTemplate {
    fn can_add_restriction(&self) -> bool {
        !self.selected_restrictions.iter().any(|r| r.hour.is_none())
    }

    fn is_restriction_hour_available(&self, hour: i64) -> bool {
        !self
            .selected_restrictions
            .iter()
            .any(|r| r.hour == Some(hour))
    }
}

pub async fn day_details_response(state: AppState, date: Date) -> HttpResult {
    let selected_holiday = get_holiday(&state.read_pool, date).await?;
    let selected_tournament = get_tournament_day(&state.read_pool, date).await?;
    let selected_restrictions = get_restrictions_for_day(&state.read_pool, date).await?;
    let events = DayEvents {
        has_holiday: selected_holiday.is_some(),
        has_tournament: selected_tournament.is_some(),
        has_restriction: !selected_restrictions.is_empty(),
    };

    let mut fake_user = User::default();
    fake_user.admin_panel_access = true;
    let reservations = HoursTemplate::create_response(&state, date, &fake_user, false).await;

    DayDetailsTemplate {
        current_date: local_date(),
        selected_date: date,
        selected_holiday,
        selected_tournament,
        selected_restrictions,
        day_structure: DayStructure::fetch_or_default(&state.read_pool, date, &state.location)
            .await?,
        events,
        reservations,
    }
    .try_into_response()
}
