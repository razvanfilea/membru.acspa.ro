use crate::http::AppState;
use crate::model::day_structure::DayStructure;
use crate::model::restriction::Restriction;
use crate::utils::CssColor;
use itertools::{Either, Itertools};
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
    pub active: Vec<Reservation>,
    pub waiting: Vec<Reservation>,
    pub cancelled: Vec<Reservation>,
}

pub struct ReservationsSlot {
    pub start_hour: u8,
    pub minute: Option<u8>,
    pub end_hour: u8,
    pub reservations: Result<Reservations, String>,
}

pub struct ReservationHours {
    pub description: Option<String>,
    pub hours: Vec<ReservationsSlot>,
    pub capacity: Option<u8>,
}

impl ReservationHours {
    pub async fn fetch(state: &AppState, date: Date) -> sqlx::Result<Self> {
        let day_structure =
            DayStructure::fetch_or_default(&state.read_pool, date, &state.location).await?;
        let mut tx = state.read_pool.begin().await?;
        let restrictions = query_as!(
            Restriction,
            "select date, hour, message, created_at from restrictions where date = $1 order by hour",
            date
        )
        .fetch_all(tx.as_mut())
        .await?;

        // Check if the whole day is restricted
        // Since it's ordered by hour, a null hour should be first if there is one
        if let Some(restriction) = restrictions.first().filter(|r| r.hour.is_none()) {
            return Ok(Self {
                hours: day_structure
                    .iter()
                    .map(|hour| ReservationsSlot {
                        start_hour: hour,
                        minute: None,
                        end_hour: hour + day_structure.slot_duration as u8,
                        reservations: Err(restriction.message.clone()),
                    })
                    .collect(),
                description: day_structure.description,
                capacity: None,
            });
        }

        // This specifically uses the idx_reservations_location_date_guest index
        let date_reservations = query!(
        r#"select u.name as 'name!', r.user_id, hour, has_key, as_guest, in_waiting, created_for, cancelled, ur.color as role_color
            from reservations r
            inner join users u on r.user_id = u.id
            inner join user_roles ur on u.role_id = ur.id
            where date = $1 and location = $2
            order by as_guest, created_at"#,
            date,
            state.location.id
        )
        .fetch_all(tx.as_mut())
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
                        minute: None,
                        end_hour,
                        reservations: Err(restriction.message.clone()),
                    };
                }

                let (list, cancelled): (Vec<_>, Vec<_>) = date_reservations
                    .iter()
                    .filter(|record| record.hour as u8 == hour)
                    .partition_map(|record| {
                        let res = Reservation {
                            name: record
                                .created_for
                                .clone()
                                .unwrap_or_else(|| record.name.clone()),
                            has_key: record.has_key && record.created_for.is_none(),
                            has_account: record.created_for.is_none(),
                            color: if record.as_guest {
                                CssColor::Blue
                            } else if record.created_for.is_none() {
                                CssColor::from_str(
                                    record.role_color.as_ref().map_or("", String::as_str),
                                )
                                .unwrap_or(CssColor::None)
                            } else {
                                CssColor::Pink
                            },
                            waiting: record.in_waiting,
                            user_id: record.user_id,
                            created_for: record.created_for.clone(),
                        };

                        match record.cancelled {
                            false => Either::Left(res),
                            true => Either::Right(res),
                        }
                    });

                let (active, waiting) = list.into_iter().partition(|r| !r.waiting);

                ReservationsSlot {
                    start_hour: hour,
                    minute: day_structure.slots_start_minute.map(|minute| minute as u8),
                    end_hour,
                    reservations: Ok(Reservations {
                        active,
                        waiting,
                        cancelled,
                    }),
                }
            })
            .collect();

        let capacity = DayStructure::fetch_for_date(tx.as_mut(), date)
            .await?
            .and_then(|day| day.slot_capacity.map(|capacity| capacity as u8));

        Ok(Self {
            description: day_structure.description,
            hours,
            capacity,
        })
    }
}
