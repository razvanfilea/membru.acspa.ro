use crate::http::AppState;
use crate::model::restriction::Restriction;
use crate::utils::CssColor;
use crate::utils::queries::{get_alt_day_structure_for_day, get_day_structure};
use sqlx::{query, query_as};
use std::str::FromStr;
use time::Date;

pub struct Reservation {
    pub name: String,
    pub has_key: bool,
    pub has_account: bool,
    pub color: CssColor,
    pub waiting: bool,

    pub user_id: i64,
    pub created_for: Option<String>,
}

pub struct Reservations {
    pub list: Vec<Reservation>,
    pub waiting: Vec<Reservation>,
}

pub struct ReservationsSlot {
    pub start_hour: u8,
    pub end_hour: u8,
    pub reservations: Result<Reservations, String>,
}

pub struct ReservationHours {
    pub description: Option<String>,
    pub hours: Vec<ReservationsSlot>,
    pub capacity: Option<u8>,
}

pub async fn get_reservation_hours(
    state: &AppState,
    date: Date,
) -> Result<ReservationHours, sqlx::Error> {
    let day_structure = get_day_structure(state, date).await;
    let restrictions = query_as!(
        Restriction,
        "select date, hour, message, created_at from restrictions where date = $1 order by hour",
        date
    )
    .fetch_all(&state.read_pool)
    .await?;

    // Check if the whole day is restricted
    // Since it's ordered by hour, a null hour should be first if there is one
    if let Some(restriction) = restrictions.first().filter(|r| r.hour.is_none()) {
        return Ok(ReservationHours {
            hours: day_structure
                .iter()
                .map(|hour| ReservationsSlot {
                    start_hour: hour,
                    end_hour: hour + day_structure.slot_duration as u8,
                    reservations: Err(restriction.message.clone()),
                })
                .collect(),
            description: day_structure.description,
            capacity: None,
        });
    }

    // This specifically uses the idx_reservations_date_cancelled index
    let date_reservations = query!(
        r#"select u.name as 'name!', r.user_id, hour, has_key, as_guest, in_waiting, created_for, ur.color as role_color
        from reservations r
        inner join users u on r.user_id = u.id
        inner join user_roles ur on u.role_id = ur.id
        where date = $1 and cancelled = false
        order by as_guest, created_at"#,
        date
    )
    .fetch_all(&state.read_pool)
    .await?;

    let hours = day_structure
        .iter()
        .map(|hour| {
            let end_hour = hour + day_structure.slot_duration as u8;

            if let Some(restriction) = restrictions
                .iter()
                .find(|restriction| restriction.hour == Some(hour as i64))
            {
                return ReservationsSlot {
                    start_hour: hour,
                    end_hour,
                    reservations: Err(restriction.message.clone()),
                };
            }

            let (list, waiting) = date_reservations
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| Reservation {
                    name: record
                        .created_for
                        .clone()
                        .unwrap_or_else(|| record.name.clone()),
                    has_key: record.has_key && record.created_for.is_none(),
                    has_account: record.created_for.is_none(),
                    color: if record.as_guest {
                        CssColor::Blue
                    } else if record.created_for.is_none() {
                        CssColor::from_str(record.role_color.as_ref().map_or("", String::as_str))
                            .unwrap_or(CssColor::None)
                    } else {
                        CssColor::Pink
                    },
                    waiting: record.in_waiting,
                    user_id: record.user_id,
                    created_for: record.created_for.clone(),
                })
                .partition(|r| !r.waiting);

            ReservationsSlot {
                start_hour: hour,
                end_hour,
                reservations: Ok(Reservations { list, waiting }),
            }
        })
        .collect();

    let capacity = get_alt_day_structure_for_day(&state.read_pool, date)
        .await
        .and_then(|day| day.slot_capacity.map(|capacity| capacity as u8));

    Ok(ReservationHours {
        description: day_structure.description,
        hours,
        capacity,
    })
}
