ALTER TABLE user_roles ADD COLUMN monthly_fee INTEGER;

DROP VIEW IF EXISTS users_with_role;
CREATE VIEW users_with_role AS
SELECT u.*,
       r.name AS role,
       r.admin_panel_access,
       r.monthly_fee
FROM users u
         INNER JOIN user_roles r ON u.role_id = r.id
WHERE is_deleted = FALSE;
