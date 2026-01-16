use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, UtcOffset};

pub const ISO_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]-[month]-[day]");

pub const READABLE_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year]");

pub const MONTH_YEAR: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year].[month]");

pub const READABLE_DATE_TIME: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year] [hour]:[minute]");

pub trait DateFormatExt {
    fn to_iso(&self) -> String;
    fn to_readable(&self) -> String;
    fn to_month_year(&self) -> String;
}

impl DateFormatExt for Date {
    fn to_iso(&self) -> String {
        self.format(ISO_DATE).unwrap_or_default()
    }

    fn to_readable(&self) -> String {
        self.format(READABLE_DATE).unwrap_or_default()
    }

    fn to_month_year(&self) -> String {
        self.format(MONTH_YEAR).unwrap_or_default()
    }
}

impl DateFormatExt for Option<Date> {
    fn to_iso(&self) -> String {
        self.map(|d| d.to_iso()).unwrap_or_default()
    }

    fn to_readable(&self) -> String {
        self.map(|d| d.to_readable())
            .unwrap_or_else(|| "?".to_string())
    }

    fn to_month_year(&self) -> String {
        self.map(|d| d.to_month_year())
            .unwrap_or_else(|| "-".to_string())
    }
}

pub fn as_local(time: &OffsetDateTime) -> String {
    let offset = UtcOffset::current_local_offset().expect("Failed to determine local offset");
    time.to_offset(offset).format(READABLE_DATE_TIME).unwrap()
}

pub fn month_as_str(month: &Month) -> &'static str {
    match month {
        Month::January => "Ianuarie",
        Month::February => "Februarie",
        Month::March => "Martie",
        Month::April => "Aprilie",
        Month::May => "Mai",
        Month::June => "Iunie",
        Month::July => "Iulie",
        Month::August => "August",
        Month::September => "Septembrie",
        Month::October => "Octombrie",
        Month::November => "Noiembrie",
        Month::December => "Decembrie",
    }
}
