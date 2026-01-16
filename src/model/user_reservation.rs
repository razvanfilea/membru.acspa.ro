use crate::model::location::Location;
use crate::model::user::User;
use crate::utils::dates::YearMonth;
use crate::utils::local_time;
use itertools::Itertools;
use sqlx::{SqliteExecutor, query, query_as};
use time::{Date, Month, OffsetDateTime};
pub struct UserReservation {
    pub date: Date,
    pub hour: i64,

    pub as_guest: bool,

    pub cancelled: bool,
    pub in_waiting: bool,

    pub created_at: OffsetDateTime,
}

impl UserReservation {
    pub fn is_cancellable(&self) -> bool {
        let now = local_time();
        let now_date = now.date();
        !self.cancelled
            && (self.date > now_date
                || (self.date == now_date && self.hour as u8 >= now.time().hour()))
    }

    pub async fn delete_on_day(
        executor: impl SqliteExecutor<'_>,
        date: Date,
        hour: Option<u8>,
    ) -> sqlx::Result<u64> {
        query!(
            "delete from reservations where date = $1 and ($2 is null or hour = $2)",
            date,
            hour
        )
        .execute(executor)
        .await
        .map(|result| result.rows_affected())
    }
}

pub struct GroupedUserReservations {
    pub year: i32,
    pub month: Month,
    pub reservations: Vec<UserReservation>,
}

impl GroupedUserReservations {
    pub async fn fetch_for_user(
        executor: impl SqliteExecutor<'_>,
        user_id: i64,
        cancelled: bool,
    ) -> sqlx::Result<Vec<Self>> {
        let reservations = query_as!(
            UserReservation,
            "select r.date, r.hour, r.as_guest, r.cancelled, r.in_waiting, r.created_at from reservations as r
             where user_id = $1 and cancelled = $2 and created_for is null",
            user_id,
            cancelled
        ).fetch_all(executor)
            .await?;

        Ok(reservations
            .into_iter()
            .into_group_map_by(|res| YearMonth {
                year: res.date.year(),
                month: res.date.month(),
            })
            .into_iter()
            .map(|(year_month, reservations)| Self {
                year: year_month.year,
                month: year_month.month,
                reservations,
            })
            .sorted_by(|a, b| {
                a.year
                    .cmp(&b.year)
                    .then((a.month as u8).cmp(&(b.month as u8)))
                    .reverse()
            })
            .collect())
    }
}

#[derive(Debug, Default)]
pub struct ReservationsCount {
    pub member: i64,
    pub guest: i64,
}

impl ReservationsCount {
    pub async fn fetch_for_slot(
        executor: impl SqliteExecutor<'_>,
        location: &Location,
        date: Date,
        hour: u8,
    ) -> sqlx::Result<Self> {
        let counts = query!(
            "select as_guest, count(*) as 'count!: i64' from reservations
            where location = $1 and date = $2 and hour = $3 and cancelled = false and in_waiting = false
            group by as_guest",
            location.id,
            date,
            hour
        )
        .fetch_all(executor)
        .await?;

        let mut result = Self::default();

        for row in counts {
            if row.as_guest {
                result.guest = row.count;
            } else {
                result.member = row.count;
            }
        }

        Ok(result)
    }

    pub async fn fetch_user_week(
        executor: impl SqliteExecutor<'_>,
        user: &User,
        date: Date,
    ) -> sqlx::Result<Self> {
        let counts = query!(
            "select r.as_guest, count(*) as 'count! :i64' from reservations r
             left join alternative_days d on r.date = d.date
             where r.user_id = $1 and r.cancelled = false
             and (d.consumes_reservation is null or d.consumes_reservation = true)
             and strftime('%Y%W', r.date) = strftime('%Y%W', $2)
             group by r.as_guest",
            user.id,
            date
        )
        .fetch_all(executor)
        .await?;

        let mut result = Self::default();

        for row in counts {
            if row.as_guest {
                result.guest = row.count;
            } else {
                result.member = row.count;
            }
        }

        Ok(result)
    }
}
