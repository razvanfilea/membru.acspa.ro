use crate::utils::dates::YearMonth;
use time::{Date, OffsetDateTime};

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

impl PaymentBreak {
    pub fn months(&self) -> i32 {
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
}
