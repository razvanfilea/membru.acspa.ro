use crate::http::AppState;
use crate::http::error::{HttpError, HttpResult, OrBail, OrBailWith, bail};
use crate::http::pages::AuthSession;
use crate::http::response::hx_notify;
use crate::http::template_into_response::TemplateIntoResponse;
use crate::model::day_structure::DayStructure;
use crate::model::user::User;
use crate::reservation::{self, ReservationSuccess};
use crate::utils::date_formats::{self, DateFormatExt, IsoDate};
use crate::utils::{local_date, local_time};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::{SqlitePool, query, query_as};
use time::{Date, OffsetDateTime};
use tracing::info;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(guests_page))
        .route("/", put(create_guest))
        .route("/list", get(guests_list))
        .route("/{rowid}", delete(delete_guest))
        .route("/select_hour", post(select_hour))
}

/// Pages showing guests reload them when this event is sent
const GUESTS_CHANGED_EVENT: &str = "guests-changed";

const MIN_GUEST_NAME_LEN: usize = 3;
const MAX_GUEST_NAME_LEN: usize = 60;

pub(super) struct GuestDto {
    pub(super) rowid: i64,
    pub(super) name: String,
    pub(super) date: Date,
    pub(super) hour: i64,
    pub(super) start_minute: Option<i64>,
    pub(super) as_guest: bool,
    pub(super) in_waiting: bool,
    pub(super) created_by: String,
    pub(super) created_at: OffsetDateTime,
}

impl GuestDto {
    pub(super) fn is_cancellable(&self) -> bool {
        reservation::is_cancellable_at(self.date, self.hour as u8, self.start_minute, local_time())
    }

    pub(super) async fn fetch(pool: &SqlitePool, date: Option<Date>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            GuestDto,
            r#"select r._rowid_ as 'rowid!', r.created_for 'name!', r.date, r.hour, a.slots_start_minute as 'start_minute?', r.as_guest, r.in_waiting, r.created_at, u.name as created_by
            from reservations r
            inner join users u on r.user_id = u.id
            left join alternative_days a on a.date = r.date
            where r.created_for is not null and ($1 is null or r.date = $1)
            order by r.date desc, r.hour, r.created_at desc"#,
            date
        )
        .fetch_all(pool)
        .await
    }
}

#[derive(Template)]
#[template(path = "admin/guests/guests_page.html", block = "list")]
struct GuestsListTemplate {
    guests: Vec<GuestDto>,
}

async fn guests_page(State(state): State<AppState>, auth_session: AuthSession) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/guests/guests_page.html")]
    struct GuestsTemplate {
        user: User,
        current_date: Date,
        guests: Vec<GuestDto>,
    }

    GuestsTemplate {
        user: auth_session.user().await.ok_or(HttpError::Unauthorized)?,
        guests: GuestDto::fetch(&state.read_pool, None).await?,
        current_date: local_date(),
    }
    .try_into_response()
}

async fn guests_list(State(state): State<AppState>) -> HttpResult {
    GuestsListTemplate {
        guests: GuestDto::fetch(&state.read_pool, None).await?,
    }
    .try_into_response()
}

#[derive(Deserialize)]
struct SelectDateForm {
    date: IsoDate,
}

async fn select_hour(
    State(state): State<AppState>,
    Form(form): Form<SelectDateForm>,
) -> HttpResult {
    #[derive(Template)]
    #[template(path = "admin/guests/select_hour.html")]
    struct SelectHourTemplate {
        hours: Vec<u8>,
    }

    SelectHourTemplate {
        hours: guest_hours(&state, *form.date).await?,
    }
    .try_into_response()
}

/// Hours of `date` that haven't started yet, guests can only be added to (and removed from) those
pub(super) async fn guest_hours(state: &AppState, date: Date) -> sqlx::Result<Vec<u8>> {
    let now = local_time();
    let structure = DayStructure::fetch_or_default(&state.read_pool, date, &state.location).await?;
    Ok(structure
        .iter()
        .filter(|hour| {
            reservation::is_cancellable_at(date, *hour, structure.slots_start_minute, now)
        })
        .collect())
}

fn guests_changed(message: &str) -> HttpResult {
    Ok(([("HX-Trigger", GUESTS_CHANGED_EVENT)], hx_notify(message)?).into_response())
}

#[derive(Deserialize)]
struct NewGuestForm {
    name: String,
    date: IsoDate,
    hour: u8,
    special: Option<String>,
}

async fn create_guest(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(guest): Form<NewGuestForm>,
) -> HttpResult {
    let user = auth_session.user().await.ok_or(HttpError::Unauthorized)?;
    let date = *guest.date;
    let hour = guest.hour;
    let name = guest.name.trim();

    let name_len = name.chars().count();
    if !(MIN_GUEST_NAME_LEN..=MAX_GUEST_NAME_LEN).contains(&name_len) {
        return Err(bail(format!(
            "Numele invitatului trebuie să aibă între {MIN_GUEST_NAME_LEN} și {MAX_GUEST_NAME_LEN} caractere"
        )));
    }

    let structure = DayStructure::fetch_or_default(&state.read_pool, date, &state.location).await?;
    if !reservation::is_cancellable_at(date, hour, structure.slots_start_minute, local_time()) {
        return Err(bail(
            "Nu se pot adăuga invitați în orele începute sau trecute",
        ));
    }

    let referral = reservation::Referral {
        is_special: guest.special.is_some(),
        created_for: name,
    };
    let success = reservation::create_reservation(
        &state.write_pool,
        &state.location,
        local_time(),
        &user,
        date,
        hour,
        Some(referral),
    )
    .await
    .or_bail_with(|e| format!("Nu s-a putut adăuga invitatul: {e}"))?;

    info!("Add guest with date: {date}, hour: {hour}: {referral:?}");
    let _ = state.reservation_notifier.send(());

    guests_changed(match success {
        ReservationSuccess::InWaiting { .. } => "Ora este plină, invitatul este în așteptare",
        _ => "Invitatul a fost adăugat",
    })
}

#[derive(Deserialize)]
struct DeleteGuestQuery {
    name: String,
}

async fn delete_guest(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(rowid): Path<i64>,
    Query(query): Query<DeleteGuestQuery>,
) -> HttpResult {
    let user = auth_session.user().await.ok_or(HttpError::Unauthorized)?;

    let mut tx = state.write_pool.begin().await?;

    // Rowids are reused after deleting the newest row, the name makes sure a stale page
    // doesn't delete a guest added since
    let guest = query!(
        r#"select date as "date: Date", hour as "hour: u8", user_id, created_for as "created_for!"
        from reservations where _rowid_ = $1 and created_for = $2 and location = $3"#,
        rowid,
        query.name,
        state.location.id
    )
    .fetch_optional(tx.as_mut())
    .await?
    .or_bail("Invitatul nu există")?;

    let structure =
        DayStructure::fetch_or_default(tx.as_mut(), guest.date, &state.location).await?;
    if !reservation::is_cancellable_at(
        guest.date,
        guest.hour,
        structure.slots_start_minute,
        local_time(),
    ) {
        return Err(bail(
            "Invitații din orele începute sau trecute nu mai pot fi șterși",
        ));
    }

    let cancelled = reservation::cancel_reservation(
        tx,
        &state.location,
        guest.date,
        guest.hour,
        guest.user_id,
        Some(&guest.created_for),
        user.id,
    )
    .await?;
    if !cancelled {
        return Err(bail("Invitatul nu există"));
    }

    info!(
        "Delete guest {} on {} hour {}",
        guest.created_for, guest.date, guest.hour
    );
    let _ = state.reservation_notifier.send(());

    guests_changed("Invitatul a fost șters")
}
