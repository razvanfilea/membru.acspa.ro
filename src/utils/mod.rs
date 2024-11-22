mod color;
pub mod date_formats;
pub mod date_iter;
pub mod queries;

pub use color::*;
use time::{OffsetDateTime, UtcOffset};

pub fn local_time() -> OffsetDateTime {
    let offset = UtcOffset::current_local_offset().expect("Failed to set Soundness to Unsound");
    OffsetDateTime::now_utc().to_offset(offset)
}
