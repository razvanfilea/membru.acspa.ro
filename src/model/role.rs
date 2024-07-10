pub struct UserRole {
    pub id: i64,
    pub name: String,
    pub max_reservations: i64,
    pub max_guest_reservations: i64,
    pub admin_panel_access: bool,
}
