use crate::model::payment::PaymentBreak;
use crate::utils::dates::YearMonth;
use sqlx::{SqliteConnection, query, query_as};
use time::{Date, Month};

pub struct PaymentContext {
    allocations: Vec<YearMonth>,
    breaks: Vec<PaymentBreak>,
}

impl PaymentContext {
    pub fn new(allocations: Vec<YearMonth>, breaks: Vec<PaymentBreak>) -> Self {
        Self {
            allocations,
            breaks,
        }
    }

    pub async fn fetch(conn: &mut SqliteConnection, user_id: i64) -> sqlx::Result<Self> {
        let breaks = query_as!(
            PaymentBreak,
            "select m.*, u.name as created_by_name
             from payment_breaks m join users u on u.id = m.created_by
             where user_id = $1 order by start_date desc",
            user_id
        )
        .fetch_all(&mut *conn)
        .await?;

        let allocations = query!(
            "select year, month from payment_allocations where payment_id in (select id from payments where user_id = ?)",
            user_id
        )
            .fetch_all(&mut *conn)
            .await
            .map(|vec| {
                vec.into_iter()
                    .filter_map(|record| {
                        Some(YearMonth::new(
                            record.year as i32,
                            Month::try_from(record.month as u8).ok()?,
                        ))
                    })
                    .collect()
            })?;

        Ok(Self::new(allocations, breaks))
    }

    pub fn is_month_paid(&self, month: YearMonth) -> bool {
        self.allocations.contains(&month)
    }

    pub fn is_month_in_break(&self, month: YearMonth) -> bool {
        let month_date = month.to_date();
        self.breaks
            .iter()
            .any(|b| (b.start_date..=b.end_date).contains(&month_date))
    }

    pub fn overlaps_existing_break(&self, start: Date, end: Date) -> bool {
        self.breaks
            .iter()
            .any(|b| start <= b.end_date && b.start_date <= end)
    }

    pub fn overlaps_existing_payment(&self, start: Date, end: Date) -> bool {
        self.allocations
            .iter()
            .any(|alloc| (start..=end).contains(&alloc.to_date()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn is_month_paid_returns_true_when_allocation_exists() {
        let ctx = PaymentContext::new(vec![YearMonth::new(2020, Month::March)], vec![]);

        assert!(ctx.is_month_paid(YearMonth::new(2020, Month::March)));
        assert!(!ctx.is_month_paid(YearMonth::new(2020, Month::April)));
    }

    #[test]
    fn is_month_in_break_returns_true_when_within_break_range() {
        let ctx = PaymentContext::new(
            vec![],
            vec![PaymentBreak::make_break(
                date!(2020 - 03 - 01),
                date!(2020 - 05 - 01),
            )],
        );

        assert!(ctx.is_month_in_break(YearMonth::new(2020, Month::March)));
        assert!(ctx.is_month_in_break(YearMonth::new(2020, Month::April)));
        assert!(ctx.is_month_in_break(YearMonth::new(2020, Month::May)));
        assert!(!ctx.is_month_in_break(YearMonth::new(2020, Month::February)));
        assert!(!ctx.is_month_in_break(YearMonth::new(2020, Month::June)));
    }

    #[test]
    fn overlaps_existing_break_detects_overlap() {
        let ctx = PaymentContext::new(
            vec![],
            vec![PaymentBreak::make_break(
                date!(2020 - 03 - 01),
                date!(2020 - 05 - 01),
            )],
        );

        // Fully overlapping
        assert!(ctx.overlaps_existing_break(date!(2020 - 03 - 01), date!(2020 - 05 - 01)));
        // Partial overlap at start
        assert!(ctx.overlaps_existing_break(date!(2020 - 02 - 01), date!(2020 - 03 - 01)));
        // Partial overlap at end
        assert!(ctx.overlaps_existing_break(date!(2020 - 05 - 01), date!(2020 - 06 - 01)));
        // Contained within
        assert!(ctx.overlaps_existing_break(date!(2020 - 04 - 01), date!(2020 - 04 - 01)));
        // No overlap before
        assert!(!ctx.overlaps_existing_break(date!(2020 - 01 - 01), date!(2020 - 02 - 01)));
        // No overlap after
        assert!(!ctx.overlaps_existing_break(date!(2020 - 06 - 01), date!(2020 - 07 - 01)));
    }

    #[test]
    fn overlaps_existing_payment_detects_overlap() {
        let ctx = PaymentContext::new(
            vec![
                YearMonth::new(2020, Month::March),
                YearMonth::new(2020, Month::April),
            ],
            vec![],
        );

        // Range includes March
        assert!(ctx.overlaps_existing_payment(date!(2020 - 02 - 01), date!(2020 - 03 - 01)));
        // Range includes April
        assert!(ctx.overlaps_existing_payment(date!(2020 - 04 - 01), date!(2020 - 05 - 01)));
        // Range includes both
        assert!(ctx.overlaps_existing_payment(date!(2020 - 03 - 01), date!(2020 - 04 - 01)));
        // No overlap
        assert!(!ctx.overlaps_existing_payment(date!(2020 - 01 - 01), date!(2020 - 02 - 01)));
        assert!(!ctx.overlaps_existing_payment(date!(2020 - 05 - 01), date!(2020 - 06 - 01)));
    }

    #[test]
    fn is_month_in_break_year_boundary_and_leap_year() {
        let ctx = PaymentContext::new(
            vec![],
            vec![PaymentBreak::make_break(
                date!(2023 - 12 - 01),
                date!(2024 - 02 - 01),
            )],
        );

        // In break: Dec 2023, Jan 2024, Feb 2024 (leap year)
        assert!(ctx.is_month_in_break(YearMonth::new(2023, Month::December)));
        assert!(ctx.is_month_in_break(YearMonth::new(2024, Month::January)));
        assert!(ctx.is_month_in_break(YearMonth::new(2024, Month::February)));
        // Not in break
        assert!(!ctx.is_month_in_break(YearMonth::new(2023, Month::November)));
        assert!(!ctx.is_month_in_break(YearMonth::new(2024, Month::March)));
    }

    #[test]
    fn overlaps_existing_break_year_boundary_and_leap_year() {
        let ctx = PaymentContext::new(
            vec![],
            vec![PaymentBreak::make_break(
                date!(2023 - 12 - 01),
                date!(2024 - 02 - 01),
            )],
        );

        // Overlaps: crossing year boundary
        assert!(ctx.overlaps_existing_break(date!(2023 - 11 - 15), date!(2024 - 01 - 15)));
        // Overlaps: includes Feb 29 2024 (leap day)
        assert!(ctx.overlaps_existing_break(date!(2024 - 02 - 01), date!(2024 - 02 - 29)));
        // No overlap: before
        assert!(!ctx.overlaps_existing_break(date!(2023 - 10 - 01), date!(2023 - 11 - 30)));
        // No overlap: after
        assert!(!ctx.overlaps_existing_break(date!(2024 - 03 - 01), date!(2024 - 04 - 30)));
    }

    #[test]
    fn overlaps_existing_payment_year_boundary_and_leap_year() {
        let ctx = PaymentContext::new(
            vec![
                YearMonth::new(2023, Month::December),
                YearMonth::new(2024, Month::January),
                YearMonth::new(2024, Month::February),
            ],
            vec![],
        );

        // Overlaps: crossing year boundary
        assert!(ctx.overlaps_existing_payment(date!(2023 - 12 - 15), date!(2024 - 01 - 15)));
        // Overlaps: includes Feb 29 2024 (leap day)
        assert!(ctx.overlaps_existing_payment(date!(2024 - 02 - 01), date!(2024 - 02 - 29)));
        // No overlap: before
        assert!(!ctx.overlaps_existing_payment(date!(2023 - 10 - 01), date!(2023 - 11 - 30)));
        // No overlap: after
        assert!(!ctx.overlaps_existing_payment(date!(2024 - 03 - 01), date!(2024 - 04 - 30)));
    }
}
