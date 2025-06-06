use crate::reservation::{ReservationError, ReservationResult, ReservationSuccess};
use strum::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, PartialEq, EnumString, EnumIter, strum::Display, AsRefStr)]
pub enum CssColor {
    None,
    Blue,
    Red,
    Pink,
    Green,
    Yellow,
    Orange,
    Violet,
    Indigo,
    Brown,
    Gray,
}

pub fn get_reservation_result_color(result: &ReservationResult) -> CssColor {
    match result {
        Ok(success) => match success {
            ReservationSuccess::Reservation { .. } => CssColor::Green,
            ReservationSuccess::Guest => CssColor::Blue,
            ReservationSuccess::InWaiting { as_guest } => {
                if *as_guest {
                    CssColor::Blue
                } else {
                    CssColor::Green
                }
            }
        },
        Err(error) => match error {
            ReservationError::AlreadyExists { .. } => CssColor::Yellow,
            _ => CssColor::Red,
        },
    }
}
