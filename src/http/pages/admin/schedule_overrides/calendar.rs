use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail};
use crate::http::pages::AuthSession;
use crate::http::pages::admin::CalendarUrl;
use crate::http::pages::admin::guests::{GuestDto, guest_hours};
use crate::http::pages::admin::schedule_overrides::holidays::get_holiday;
use crate::http::pages::admin::schedule_overrides::restrictions::get_restrictions_for_day;
use crate::http::pages::admin::schedule_overrides::tournaments::get_tournament_day;
use crate::http::pages::admin::schedule_overrides::{AlternativeDay, AlternativeDayType};
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
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use serde::Deserialize;
use sqlx::{query, query_scalar};
use std::collections::HashMap;
use time::{Date, Duration, Month};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(calendar_page_redirect))
        .route("/{year}/{month}", get(calendar_page))
        .route("/details/{date}", get(day_details_partial))
}

async fn calendar_page_redirect() -> impl IntoResponse {
    let today = local_date();
    Redirect::to(&format!(
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
    /// Confirmed reservations, without the waiting list
    reservations: i64,
    /// Confirmed guest reservations, both guest spots and named guests
    guests: i64,
}

struct DayDetails {
    today: Date,
    date: Date,
    holiday: Option<AlternativeDay>,
    tournament: Option<AlternativeDay>,
    restrictions: Vec<Restriction>,
    day_structure: DayStructure,
    events: DayEvents,
    reservations: String,
    guests: Vec<GuestDto>,
    /// Hours guests can still be added to
    guest_hours: Vec<u8>,
    // Only needed for the copy to clipboard button
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
            guests: GuestDto::fetch(&state.read_pool, Some(date)).await?,
            guest_hours: guest_hours(state, date).await?,
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
    month: Date,
    calendar_days: DateRangeIter,
    day_markers: HashMap<Date, DayEvents>,
    day: DayDetails,
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
    let month = Date::from_calendar_date(year, month_number, 1).or_bail("Data este invalidă")?;

    let last_day = month.replace_day(month_number.length(year)).unwrap();
    let next_month_date = last_day.next_day().unwrap();
    let calendar_days = DateRangeIter::weeks_in_range(month, last_day);

    let in_month = |date: Date| date.year() == year && date.month() == month_number;
    let selected_date = query
        .day
        .map(|IsoDate(day)| day)
        .filter(|day| in_month(*day))
        .unwrap_or(if in_month(today) { today } else { month });

    // Adjacent month days are shown too, so markers cover whole weeks around the month
    let day_markers = get_day_markers(
        &state,
        month - Duration::days(7),
        last_day + Duration::days(7),
    )
    .await?;

    let prev_month_date = month.previous_day().unwrap();

    CalendarTemplate {
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
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

#[derive(Deserialize)]
struct DayDetailsQuery {
    /// Set when reloading the panel after a change made elsewhere, e.g. a guest
    #[serde(default)]
    changed: bool,
}

async fn day_details_partial(
    State(state): State<AppState>,
    Path(date): Path<IsoDate>,
    Query(query): Query<DayDetailsQuery>,
) -> HttpResult {
    DayDetailsTemplate {
        current_date: local_date(),
        day: DayDetails::fetch(&state, *date).await?,
        update_cell: query.changed,
    }
    .try_into_response()
}

#[derive(Template)]
#[template(path = "admin/calendar/day_details_response.html")]
struct DayDetailsTemplate {
    current_date: Date,
    day: DayDetails,
    /// Re-render the day's calendar cell too, only after a change: replacing it on every selection
    /// would drop the keyboard focus from the clicked cell
    update_cell: bool,
}

async fn get_day_markers(
    state: &AppState,
    from: Date,
    to: Date,
) -> sqlx::Result<HashMap<Date, DayEvents>> {
    let mut markers = get_reservation_counts(state, from, to).await?;

    let alternative_days = query!(
        r#"select date as "date: Date", type as kind from alternative_days where date between $1 and $2"#,
        from,
        to
    )
    .fetch_all(&state.read_pool)
    .await?;
    for day in alternative_days {
        let events = markers.entry(day.date).or_default();
        if day.kind == AlternativeDayType::Holiday.as_ref() {
            events.has_holiday = true;
        } else if day.kind == AlternativeDayType::Tournament.as_ref() {
            events.has_tournament = true;
        }
    }

    let restricted_days = query_scalar!(
        r#"select distinct date as "date: Date" from restrictions
           where location = $1 and date between $2 and $3"#,
        state.location.id,
        from,
        to
    )
    .fetch_all(&state.read_pool)
    .await?;
    for date in restricted_days {
        markers.entry(date).or_default().has_restriction = true;
    }

    Ok(markers)
}

/// Reservation and guest counts per day, between `from` and `to` inclusive
async fn get_reservation_counts(
    state: &AppState,
    from: Date,
    to: Date,
) -> sqlx::Result<HashMap<Date, DayEvents>> {
    let rows = query!(
        r#"select date as "date: Date",
                  sum(not in_waiting) as "reservations!: i64",
                  sum(not in_waiting and (as_guest or created_for is not null)) as "guests!: i64"
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
                guests: row.guests,
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

    let participants = query_scalar!(
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

    Ok(participants.join("\n"))
}

/// Open home pages are told to reload the day's hours too
pub(super) async fn day_details_response(state: AppState, date: Date) -> HttpResult {
    let _ = state.reservation_notifier.send(());
    DayDetailsTemplate {
        current_date: local_date(),
        day: DayDetails::fetch(&state, date).await?,
        update_cell: true,
    }
    .try_into_response()
}
