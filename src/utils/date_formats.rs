use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

pub const ISO_DATE_UNDERLINE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]_[month]_[day]");

pub const ISO_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]-[month]-[day]");

pub const READABLE_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year]");

pub fn format_as_local(time: &OffsetDateTime) -> String {
    let offset = UtcOffset::current_local_offset().expect("Failed to determine local offset");
     time.to_offset(offset).format(READABLE_DATE_TIME).unwrap()
}

pub const READABLE_DATE_TIME: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year] [hour]:[minute]");
