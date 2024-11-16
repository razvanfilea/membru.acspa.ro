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
