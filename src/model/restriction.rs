/// A restriction of one day, for one hour or the whole day when `hour` is `None`
pub struct Restriction {
    pub hour: Option<i64>,
    pub message: String,
}
