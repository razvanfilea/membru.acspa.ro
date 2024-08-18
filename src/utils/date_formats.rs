use time::macros::format_description;

pub const ISO_DATE_UNDERLINE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]_[month]_[day]");

pub const ISO_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[year]-[month]-[day]");

pub const READABLE_DATE: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year]");

pub const READABLE_DATE_TIME: &[time::format_description::BorrowedFormatItem] =
    format_description!("[day].[month].[year] [hour]:[minute]");
