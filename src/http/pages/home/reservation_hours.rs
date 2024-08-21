use std::str::FromStr;
use crate::http::AppState;
use crate::model::restriction::Restriction;
use crate::utils::{get_hour_structure_for_day, CssColor};
use sqlx::{query, query_as};
use time::Date;

pub struct PossibleReservation {
    pub name: String,
    pub has_key: bool,
    pub has_account: bool,
    pub color: CssColor,
}

pub struct ReservationSlot {
    pub start_hour: u8,
    pub end_hour: u8,
    pub reservations: Result<Vec<PossibleReservation>, String>,
}

pub async fn get_reservation_hours(state: &AppState, date: Date) -> Vec<ReservationSlot> {
    let hour_structure = get_hour_structure_for_day(state, date).await;
    let restrictions = query_as!(
        Restriction,
        "select date, hour, message, created_at from reservations_restrictions where date = $1 order by hour",
        date
    )
    .fetch_all(&state.read_pool)
    .await
    .expect("Database error");

    // Check if the whole day is restricted
    if let Some(restriction) = restrictions.first().filter(|r| r.hour.is_none()) {
        return hour_structure
            .iter()
            .map(|hour| ReservationSlot {
                start_hour: hour,
                end_hour: hour + hour_structure.slot_duration as u8,
                reservations: Err(restriction.message.clone()),
            })
            .collect();
    }

    let date_reservations = query!(
        r#"select u.name as 'name!', hour, has_key, as_guest, created_for, ur.color as role_color
        from reservations r
        inner join users u on r.user_id = u.id
        inner join user_roles ur on u.role_id = ur.id
        where date = $1 order by as_guest, created_at"#,
        date
    )
    .fetch_all(&state.read_pool)
    .await
    .expect("Database error");

    hour_structure
        .iter()
        .map(|hour| {
            let end_hour = hour + hour_structure.slot_duration as u8;

            if let Some(restriction) = restrictions
                .iter()
                .find(|restriction| restriction.hour == Some(hour as i64))
            {
                return ReservationSlot {
                    start_hour: hour,
                    end_hour,
                    reservations: Err(restriction.message.clone()),
                };
            }

            let reservations = date_reservations
                .iter()
                .filter(|record| record.hour as u8 == hour)
                .map(|record| PossibleReservation {
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
                });

            ReservationSlot {
                start_hour: hour,
                end_hour,
                reservations: Ok(reservations.collect()),
            }
        })
        .collect()
}
