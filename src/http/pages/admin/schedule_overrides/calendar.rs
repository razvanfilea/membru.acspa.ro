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
use crate::reservation::is_within_history_retention;
use crate::utils::date_formats::{self, DateFormatExt, IsoDate};
use crate::utils::dates::DateRangeIter;
use crate::utils::local_date;
use askama::Template;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use std::collections::HashMap;
use time::{Date, Duration, Month};

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
    has_guests: bool,
    /// Confirmed reservations, without the waiting list
    reservations: i64,
}

/// Everything the day panel shows for the selected date
struct DayDetails {
    today: Date,
    date: Date,
    holiday: Option<AlternativeDay>,
    tournament: Option<AlternativeDay>,
    restrictions: Vec<Restriction>,
    day_structure: DayStructure,
    events: DayEvents,
    reservations: String,
    // This is needed just for the copy to clipboard function
    tournament_participants: String,
}

impl DayDetails {
    async fn fetch(state: &AppState, date: Date) -> HttpResult<Self> {
        let holiday = get_holiday(&state.read_pool, date).await?;
        let tournament = get_tournament_day(&state.read_pool, date).await?;
        let restrictions = get_restrictions_for_day(&state.read_pool, date).await?;
        let mut events = get_reservation_counts(state, date, date)
            .await?
            .remove(&date)
            .unwrap_or_default();
        events.has_holiday = holiday.is_some();
        events.has_tournament = tournament.is_some();
        events.has_restriction = !restrictions.is_empty();

        let fake_user = User {
            admin_panel_access: true,
            ..Default::default()
        };
        let reservations = HoursTemplate::create_response(state, date, &fake_user, false)
            .await
            .unwrap_or_default();

        let tournament_participants = get_tournament_participants(state, &tournament).await?;

        Ok(Self {
            today: local_date(),
            date,
            holiday,
            tournament,
            restrictions,
            day_structure: DayStructure::fetch_or_default(&state.read_pool, date, &state.location)
                .await?,
            events,
            reservations,
            tournament_participants,
        })
    }

    fn can_add_restriction(&self) -> bool {
        !self.restrictions.iter().any(|r| r.hour.is_none())
    }

    fn is_restriction_hour_available(&self, hour: i64) -> bool {
        !self.restrictions.iter().any(|r| r.hour == Some(hour))
    }

    fn is_past_date(&self) -> bool {
        self.date < self.today
    }

    fn has_history(&self) -> bool {
        is_within_history_retention(self.date, self.today)
    }

    fn weekday(&self) -> &'static str {
        date_formats::weekday_as_str(self.date.weekday())
    }
}

#[derive(Template)]
#[template(path = "admin/calendar/calendar_page.html")]
struct CalendarTemplate {
    user: User,
    current_date: Date,
    /// First day of the displayed month
    month: Date,
    calendar_days: DateRangeIter,
    day_markers: HashMap<Date, DayEvents>, // Markers for the calendar grid
    day: DayDetails,
    // Navigation
    prev_month: (i32, u8),
    next_month: (i32, u8),
}

#[derive(Deserialize)]
struct CalendarQuery {
    day: Option<IsoDate>,
}

async fn calendar_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path((year, month_u8)): Path<(i32, u8)>,
    Query(query): Query<CalendarQuery>,
) -> HttpResult {
    let today = local_date();
    let month_number = Month::try_from(month_u8).unwrap_or(today.month());
    let month = Date::from_calendar_date(year, month_number, 1).or_bail("Data este invalida")?;

    // Calculate calendar range (full weeks)
    let next_month_date = if month_number == Month::December {
        Date::from_calendar_date(year + 1, Month::January, 1).unwrap()
    } else {
        Date::from_calendar_date(year, month_number.next(), 1).unwrap()
    };
    let last_day = next_month_date.previous_day().unwrap();
    let calendar_days = DateRangeIter::weeks_in_range(month, last_day);

    let in_month = |date: Date| date.year() == year && date.month() == month_number;
    let selected_date = query
        .day
        .map(|day| *day)
        .filter(|day| in_month(*day))
        .unwrap_or(if in_month(today) { today } else { month });

    let holidays = get_holidays_for_month(&state.read_pool, month).await?;
    let tournaments = get_tournament_days(&state.read_pool, Some(month)).await?;
    let restrictions = get_restrictions_for_month(&state.read_pool, month).await?;

    // Adjacent month days are shown too, count reservations for whole weeks around the month
    let mut day_markers = get_reservation_counts(
        &state,
        month - Duration::days(7),
        last_day + Duration::days(7),
    )
    .await?;
    for h in &holidays {
        day_markers.entry(h.date).or_default().has_holiday = true;
    }
    for t in &tournaments {
        day_markers.entry(t.date).or_default().has_tournament = true;
    }
    for r in &restrictions {
        day_markers.entry(r.date).or_default().has_restriction = true;
    }

    let prev_month_date = month.previous_day().unwrap();

    CalendarTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        current_date: today,
        month,
        calendar_days,
        day_markers,
        day: DayDetails::fetch(&state, selected_date).await?,
        prev_month: (prev_month_date.year(), prev_month_date.month() as u8),
        next_month: (next_month_date.year(), next_month_date.month() as u8),
    }
    .try_into_response()
}

async fn day_details_partial(
    State(state): State<AppState>,
    Path(date): Path<IsoDate>,
) -> HttpResult {
    day_details_response(state, *date).await
}

#[derive(Template)]
#[template(path = "admin/calendar/day_details_response.html")]
struct DayDetailsTemplate {
    current_date: Date,
    day: DayDetails,
}

/// Reservation counts and guest markers per day, between `from` and `to` inclusive
async fn get_reservation_counts(
    state: &AppState,
    from: Date,
    to: Date,
) -> sqlx::Result<HashMap<Date, DayEvents>> {
    let rows = sqlx::query!(
        r#"select date as "date: Date",
                  sum(not in_waiting) as "reservations!: i64",
                  max(created_for is not null) as "has_guests!: bool"
           from reservations
           where location = $1 and date between $2 and $3 and not cancelled
           group by date"#,
        state.location.id,
        from,
        to
    )
    .fetch_all(&state.read_pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let events = DayEvents {
                reservations: row.reservations,
                has_guests: row.has_guests,
                ..Default::default()
            };
            (row.date, events)
        })
        .collect())
}

async fn get_tournament_participants(
    state: &AppState,
    tournament: &Option<AlternativeDay>,
) -> sqlx::Result<String> {
    let Some(tournament) = tournament else {
        return Ok(String::new());
    };

    let participants = sqlx::query_scalar!(
        r#"select ifnull(r.created_for, u.name) as "name: String"
           from reservations r
           inner join users u on r.user_id = u.id
           where r.date = $1 and r.location = $2 and r.cancelled = 0 and r.in_waiting = 0
           order by r.created_at"#,
        tournament.date,
        state.location.id
    )
    .fetch_all(&state.read_pool)
    .await?;

    Ok(participants
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n"))
}

pub async fn day_details_response(state: AppState, date: Date) -> HttpResult {
    DayDetailsTemplate {
        current_date: local_date(),
        day: DayDetails::fetch(&state, date).await?,
    }
    .try_into_response()
}
