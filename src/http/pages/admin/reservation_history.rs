use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult};
use crate::http::pages::AuthSession;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::user::User;
use crate::reservation::{
    HISTORY_RETENTION_DAYS, history_retention_start, is_within_history_retention,
};
use crate::utils::date_formats::{DateFormatExt, ISO_DATE, weekday_as_str};
use crate::utils::local_date;
use askama::Template;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use serde::Deserialize;
use sqlx::{SqlitePool, query_as};
use std::str::FromStr;
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator};
use time::macros::format_description;
use time::{Date, OffsetDateTime, UtcOffset};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(history_page))
}

/// Matches the event names written by the `reservation_history_*` triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, AsRefStr, sqlx::Type)]
#[strum(serialize_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum HistoryEvent {
    Created,
    Cancelled,
    Deleted,
    Expired,
    Promoted,
    MovedToWaiting,
    HourChanged,
    Updated,
}

impl HistoryEvent {
    pub fn label(&self) -> &'static str {
        match self {
            HistoryEvent::Created => "Rezervare nouă",
            HistoryEvent::Cancelled => "Retrasă",
            HistoryEvent::Deleted => "Ștearsă",
            HistoryEvent::Expired => "Expirată din așteptare",
            HistoryEvent::Promoted => "Promovată",
            HistoryEvent::MovedToWaiting => "Mutată în așteptare",
            HistoryEvent::HourChanged => "Oră schimbată",
            HistoryEvent::Updated => "Modificată",
        }
    }

    /// Label used by the filter dropdown
    pub fn filter_label(&self) -> &'static str {
        match self {
            HistoryEvent::Created => "Rezervări noi",
            HistoryEvent::Cancelled => "Retrageri",
            HistoryEvent::Deleted => "Ștergeri",
            HistoryEvent::Expired => "Expirări din așteptare",
            HistoryEvent::Promoted => "Promovări",
            HistoryEvent::MovedToWaiting => "Mutări în așteptare",
            HistoryEvent::HourChanged => "Ore schimbate",
            HistoryEvent::Updated => "Alte modificări",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            HistoryEvent::Created => "event_available",
            HistoryEvent::Cancelled => "event_busy",
            HistoryEvent::Deleted => "delete",
            HistoryEvent::Expired => "timer_off",
            HistoryEvent::Promoted => "arrow_upward",
            HistoryEvent::MovedToWaiting => "hourglass_top",
            HistoryEvent::HourChanged => "schedule",
            HistoryEvent::Updated => "edit",
        }
    }

    /// Background and text color of the event icon
    pub fn icon_class(&self) -> &'static str {
        match self {
            HistoryEvent::Created => "bg-success/15 text-success",
            HistoryEvent::Cancelled | HistoryEvent::Deleted => "bg-error/15 text-error",
            HistoryEvent::Expired => "bg-orange-500/15 text-orange-400",
            HistoryEvent::Promoted => "bg-info/15 text-info",
            HistoryEvent::MovedToWaiting => "bg-warning/15 text-warning",
            HistoryEvent::HourChanged => "bg-purple-500/15 text-purple-400",
            HistoryEvent::Updated => "bg-base-300 text-gray-400",
        }
    }

    pub fn text_class(&self) -> &'static str {
        match self {
            HistoryEvent::Created => "text-success",
            HistoryEvent::Cancelled | HistoryEvent::Deleted => "text-error",
            HistoryEvent::Expired => "text-orange-400",
            HistoryEvent::Promoted => "text-info",
            HistoryEvent::MovedToWaiting => "text-warning",
            HistoryEvent::HourChanged => "text-purple-400",
            HistoryEvent::Updated => "text-gray-400",
        }
    }
}

#[derive(Debug)]
pub struct HistoryEntry {
    pub event: HistoryEvent,
    pub user_id: i64,
    pub user_name: String,
    pub date: Date,
    pub hour: i64,
    pub old_hour: Option<i64>,
    pub created_for: Option<String>,
    pub as_guest: bool,
    pub waiting_position: Option<i64>,
    pub changed_by: Option<i64>,
    pub changed_by_name: Option<String>,
    pub changed_at: OffsetDateTime,
}

impl HistoryEntry {
    /// A member cancelling their own reservation on the same (local) day as the reservation itself
    pub fn is_late_cancellation(&self) -> bool {
        let offset = UtcOffset::current_local_offset().expect("Failed to determine local offset");
        self.is_cancelled_on_reservation_day(offset)
    }

    fn is_cancelled_on_reservation_day(&self, offset: UtcOffset) -> bool {
        self.event == HistoryEvent::Cancelled
            && self.changed_by == Some(self.user_id)
            && self.changed_at.to_offset(offset).date() == self.date
    }

    pub fn has_badges(&self) -> bool {
        self.as_guest || self.waiting_position.is_some() || self.is_late_cancellation()
    }

    pub fn changed_time(&self) -> String {
        self.local_changed_at().format(TIME).unwrap_or_default()
    }

    fn local_changed_at(&self) -> OffsetDateTime {
        let offset = UtcOffset::current_local_offset().expect("Failed to determine local offset");
        self.changed_at.to_offset(offset)
    }
}

const TIME: &[time::format_description::BorrowedFormatItem] =
    format_description!("[hour]:[minute]");

/// Entries changed on the same local day, newest first
pub struct HistoryDay {
    pub title: String,
    pub entries: Vec<HistoryEntry>,
}

fn group_by_day(entries: Vec<HistoryEntry>, today: Date) -> Vec<HistoryDay> {
    let mut days: Vec<(Date, Vec<HistoryEntry>)> = Vec::new();
    for entry in entries {
        let day = entry.local_changed_at().date();
        match days.last_mut() {
            Some((last_day, day_entries)) if *last_day == day => day_entries.push(entry),
            _ => days.push((day, vec![entry])),
        }
    }

    days.into_iter()
        .map(|(day, entries)| {
            let title = if day == today {
                format!("Astăzi, {}", day.to_readable())
            } else if day.next_day() == Some(today) {
                format!("Ieri, {}", day.to_readable())
            } else {
                format!("{}, {}", weekday_as_str(day.weekday()), day.to_readable())
            };
            HistoryDay { title, entries }
        })
        .collect()
}

pub struct HistoryFilters {
    pub user_id: Option<i64>,
    pub date: Option<Date>,
    pub event: Option<HistoryEvent>,
}

impl HistoryFilters {
    pub fn is_user_selected(&self, id: &i64) -> bool {
        self.user_id == Some(*id)
    }

    pub fn is_event_selected(&self, event: &HistoryEvent) -> bool {
        self.event == Some(*event)
    }
}

pub async fn fetch_history(
    pool: &SqlitePool,
    filters: &HistoryFilters,
) -> sqlx::Result<Vec<HistoryEntry>> {
    let event = filters.event.as_ref().map(|e| e.as_ref());
    query_as!(
        HistoryEntry,
        r#"select h.event as "event: HistoryEvent", h.user_id, coalesce(u.name, '?') as "user_name!: String",
            h.date, h.hour, h.old_hour, h.created_for, h.as_guest, h.waiting_position,
            h.changed_by, a.name as "changed_by_name?: String", h.changed_at
        from reservation_history h
        left join users u on u.id = h.user_id
        left join users a on a.id = h.changed_by
        -- Old rows are purged periodically, this keeps the page exact between purges
        where h.changed_at >= datetime('now', '-' || $1 || ' days')
            and ($2 is null or h.user_id = $2)
            and ($3 is null or h.date = $3)
            and ($4 is null or h.event = $4)
        order by h.changed_at desc, h.id desc
        limit 500"#,
        HISTORY_RETENTION_DAYS,
        filters.user_id,
        filters.date,
        event
    )
    .fetch_all(pool)
    .await
}

pub struct UserOption {
    id: i64,
    name: String,
}

/// Form fields arrive as empty strings when nothing is selected
#[derive(Deserialize)]
struct HistoryQuery {
    user_id: Option<String>,
    date: Option<String>,
    event: Option<String>,
}

impl HistoryQuery {
    fn into_filters(self) -> HistoryFilters {
        let non_empty = |value: Option<String>| value.filter(|v| !v.trim().is_empty());
        HistoryFilters {
            user_id: non_empty(self.user_id).and_then(|id| id.parse().ok()),
            date: non_empty(self.date).and_then(|date| Date::parse(&date, ISO_DATE).ok()),
            event: non_empty(self.event).and_then(|event| HistoryEvent::from_str(&event).ok()),
        }
    }
}

async fn history_page(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Query(query): Query<HistoryQuery>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/history/history_page.html")]
    struct HistoryTemplate {
        user: User,
        users: Vec<UserOption>,
        events: Vec<HistoryEvent>,
        filters: HistoryFilters,
        retention_days: i64,
        date_beyond_retention: bool,
        min_date: Date,
        total: usize,
        days: Vec<HistoryDay>,
    }

    let today = local_date();
    let filters = query.into_filters();
    let date_beyond_retention = filters
        .date
        .is_some_and(|date| !is_within_history_retention(date, today));
    let entries = fetch_history(&state.read_pool, &filters).await?;
    let users = query_as!(
        UserOption,
        "select id, name from users where is_deleted = false order by name"
    )
    .fetch_all(&state.read_pool)
    .await?;

    HistoryTemplate {
        user: auth_session.user.ok_or(HttpError::Unauthorized)?,
        users,
        events: HistoryEvent::iter().collect(),
        filters,
        retention_days: HISTORY_RETENTION_DAYS,
        date_beyond_retention,
        min_date: history_retention_start(today),
        total: entries.len(),
        days: group_by_day(entries, today),
    }
    .try_into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, datetime, offset};

    fn cancelled_entry(date: Date, changed_at: OffsetDateTime) -> HistoryEntry {
        HistoryEntry {
            event: HistoryEvent::Cancelled,
            user_id: 1,
            user_name: "Test".to_string(),
            date,
            hour: 18,
            old_hour: None,
            created_for: None,
            as_guest: false,
            waiting_position: None,
            changed_by: Some(1),
            changed_by_name: None,
            changed_at,
        }
    }

    #[test]
    fn late_cancellation_is_own_cancel_on_same_local_day() {
        let offset = offset!(+3);
        let date = date!(2026 - 09 - 30);

        let same_day = cancelled_entry(date, datetime!(2026-09-30 10:00 UTC));
        assert!(same_day.is_cancelled_on_reservation_day(offset));

        // 22:30 UTC on the 29th is already the 30th in local time
        let local_midnight = cancelled_entry(date, datetime!(2026-09-29 22:30 UTC));
        assert!(local_midnight.is_cancelled_on_reservation_day(offset));

        let day_before = cancelled_entry(date, datetime!(2026-09-29 12:00 UTC));
        assert!(!day_before.is_cancelled_on_reservation_day(offset));

        let mut not_cancelled = cancelled_entry(date, datetime!(2026-09-30 10:00 UTC));
        not_cancelled.event = HistoryEvent::Deleted;
        assert!(!not_cancelled.is_cancelled_on_reservation_day(offset));

        let mut by_admin = cancelled_entry(date, datetime!(2026-09-30 10:00 UTC));
        by_admin.changed_by = Some(2);
        assert!(!by_admin.is_cancelled_on_reservation_day(offset));
    }
}
