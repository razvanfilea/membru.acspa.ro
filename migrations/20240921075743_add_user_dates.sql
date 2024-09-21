ALTER TABLE users
ADD birthday DATE;

ALTER TABLE users
ADD member_since DATE;

ALTER TABLE users
ADD received_gift DATE;

DROP VIEW users_with_role;

CREATE VIEW users_with_role AS
SELECT u.*,
       r.name AS role,
       r.admin_panel_access
FROM users u
         INNER JOIN user_roles r ON u.role_id = r.id;
