mod color;
pub mod date_formats;
pub mod date_iter;
pub mod queries;

pub use color::*;
use time::OffsetDateTime;

pub fn local_time() -> OffsetDateTime {
    OffsetDateTime::now_local().expect("Failed to determine local offset")
}
