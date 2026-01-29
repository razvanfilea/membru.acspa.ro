use crate::utils::dates::YearMonth;
use sqlx::{SqliteExecutor, SqlitePool, query, query_as};
use time::{Date, Month, OffsetDateTime};

#[derive(Debug, Clone)]
pub struct PaymentBreak {
    pub id: i64,
    #[allow(dead_code)]
    pub user_id: i64,
    pub start_date: Date,
    pub end_date: Date,
    pub reason: Option<String>,
    pub created_at: OffsetDateTime,
    pub created_by: i64,
    pub created_by_name: String,
}

#[cfg(test)]
impl PaymentBreak {
    pub fn make_break(start_date: Date, end_date: Date) -> Self {
        Self {
            id: 1,
            user_id: 1,
            start_date,
            end_date,
            reason: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            created_by: 1,
            created_by_name: "Admin".to_string(),
        }
    }
}

impl PaymentBreak {
    pub async fn fetch_for_user(
        executor: impl SqliteExecutor<'_>,
        user_id: i64,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
            PaymentBreak,
            "select m.*, u.name as created_by_name
             from payment_breaks m join users u on u.id = m.created_by
             where user_id = $1 order by start_date desc",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub fn length_in_months(&self) -> i32 {
        (self.end_date.year() - self.start_date.year()) * 12
            + (self.end_date.month() as u8 as i32 - self.start_date.month() as u8 as i32)
            + 1
    }
}

#[derive(Debug, Clone)]
pub struct PaymentWithAllocations {
    pub id: i64,
    pub amount: i64,
    pub payment_date: Date,
    pub notes: Option<String>,
    pub allocations: Vec<YearMonth>,
    pub created_at: OffsetDateTime,
    #[allow(dead_code)]
    pub created_by: i64,
    pub created_by_name: String,
}

impl PaymentWithAllocations {
    pub fn display_amount(&self) -> String {
        if self.amount % 100 == 0 {
            (self.amount / 100).to_string()
        } else {
            format!("{:.02}", self.amount as f64 / 100.0)
        }
    }

    pub async fn fetch_for_user(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<Self>> {
        let payments = query!(
            "select p.id, amount, payment_date, notes, created_at, created_by, u.name as created_by_name from payments p
             join users u on u.id = p.created_by
             where user_id = $1 order by payment_date desc",
            user_id
        )
        .fetch_all(pool)
        .await?;

        let all_allocations = query!(
            "select payment_id, year as 'year: i32', month as 'month: u8' from payment_allocations
             where payment_id in (select id from payments where user_id = ?)
             order by year desc, month desc",
            user_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(payments
            .into_iter()
            .map(|p| {
                let allocations = all_allocations
                    .iter()
                    .filter(|a| a.payment_id == p.id)
                    .filter_map(|a| Some(YearMonth::new(a.year, Month::try_from(a.month).ok()?)))
                    .collect();

                PaymentWithAllocations {
                    id: p.id,
                    amount: p.amount,
                    payment_date: p.payment_date,
                    notes: p.notes.filter(|notes| !notes.is_empty()),
                    created_at: p.created_at,
                    created_by: p.created_by,
                    created_by_name: p.created_by_name,
                    allocations,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn length_in_months() {
        let cases = [
            (date!(2024 - 01 - 01), date!(2024 - 01 - 31), 1), // same month
            (date!(2024 - 01 - 15), date!(2024 - 02 - 10), 2), // two consecutive months
            (date!(2024 - 01 - 01), date!(2024 - 12 - 31), 12), // full year
            (date!(2023 - 11 - 01), date!(2024 - 02 - 28), 4), // across years
            (date!(2022 - 06 - 01), date!(2024 - 06 - 30), 25), // multiple years
        ];
        for (start, end, expected) in cases {
            let pb = PaymentBreak::make_break(start, end);
            assert_eq!(pb.length_in_months(), expected, "from {start} to {end}");
        }
    }

    #[test]
    fn display_amount() {
        let make_payment = |amount: i64| PaymentWithAllocations {
            id: 1,
            amount,
            payment_date: date!(2024 - 01 - 01),
            notes: None,
            allocations: vec![],
            created_at: OffsetDateTime::UNIX_EPOCH,
            created_by: 1,
            created_by_name: "Admin".to_string(),
        };

        assert_eq!(make_payment(10000).display_amount(), "100");
        assert_eq!(make_payment(5000).display_amount(), "50");
        assert_eq!(make_payment(100).display_amount(), "1");
        assert_eq!(make_payment(1050).display_amount(), "10.50");
        assert_eq!(make_payment(9999).display_amount(), "99.99");
        assert_eq!(make_payment(1).display_amount(), "0.01");
    }
}
