use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum ReservationSuccess {
    Reservation { deletes_guest: bool },
    Guest,
    InWaiting { as_guest: bool },
}

#[derive(Debug, PartialEq)]
pub enum ReservationError {
    AlreadyExists { cancelled: bool },
    Restriction(String),
    DatabaseError(String),
    NoMoreReservations,
    InvalidHour,
    PastDate,
    BookingWindowClosed,
}

pub type ReservationResult<T = ReservationSuccess> = Result<T, ReservationError>;

impl From<sqlx::Error> for ReservationError {
    fn from(value: sqlx::Error) -> Self {
        ReservationError::DatabaseError(value.to_string())
    }
}

impl Display for ReservationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReservationError::AlreadyExists { cancelled } => {
                if *cancelled {
                    write!(f, "Nu te poți reînscrie după ce ai anulat o rezervare")
                } else {
                    write!(f, "Ai făcut deja o astfel de rezervare.")
                }
            }
            ReservationError::Restriction(message) => write!(f, "{}", message),
            ReservationError::DatabaseError(e) => write!(
                f,
                "Eroare cu baza de date, trimite te rog un screenshot cu aceasta eroare: {}",
                e
            ),
            ReservationError::NoMoreReservations => {
                write!(f, "Ți-ai epuizat rezervările pe săptămâna aceasta")
            }
            ReservationError::InvalidHour => write!(f, "Ora pentru rezervare nu este validă"),
            ReservationError::PastDate => {
                write!(f, "Nu poți face o rezervare pentru o zi din trecut")
            }
            ReservationError::BookingWindowClosed => {
                write!(f, "Rezervările se fac cu cel putin o oră înainte")
            }
        }
    }
}
