pub struct UserRole {
    pub id: i64,
    pub name: String,
    pub reservations: i64,
    pub guest_reservations: i64,
    pub color: Option<String>,
    pub admin_panel_access: bool,
}
