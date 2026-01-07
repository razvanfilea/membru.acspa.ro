use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, UtcOffset};

pub const ISO_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]-[month]-[day]");

pub const READABLE_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year]");

pub const READABLE_DATE_TIME: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year] [hour]:[minute]");

pub const MONTH_YEAR: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year].[month]");

pub fn as_iso(date: &Date) -> String {
    date.format(ISO_DATE).unwrap()
}

pub fn as_readable(date: &Date) -> String {
    date.format(READABLE_DATE).unwrap()
}

/*pub fn as_readable_with_time(date: &PrimitiveDateTime) -> String {
    date.format(READABLE_DATE_TIME).unwrap()
}*/

pub fn as_month_year(date: &Date) -> String {
    date.format(MONTH_YEAR).unwrap()
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
