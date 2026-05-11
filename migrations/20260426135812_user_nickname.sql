ALTER TABLE users ADD COLUMN nickname TEXT;

DROP VIEW IF EXISTS users_with_role;
CREATE VIEW users_with_role AS
SELECT u.id,
       u.email,
       u.name,
       u.nickname,
       u.password_hash,
       ur.name AS role,
       u.is_active,
       ur.admin_panel_access
FROM users u
         JOIN user_roles ur ON u.role_id = ur.id
WHERE u.is_deleted = false;

DROP VIEW IF EXISTS user_details_with_role;
CREATE VIEW user_details_with_role AS
SELECT u.id,
       u.email,
       u.name,
       u.nickname,
       ur.name AS role,
       u.is_active,
       u.has_key,
       ur.admin_panel_access,
       u.member_since,
       u.birthday,
       u.received_gift,
       ur.monthly_fee
FROM users u
         JOIN user_roles ur ON u.role_id = ur.id
WHERE u.is_deleted = false;
