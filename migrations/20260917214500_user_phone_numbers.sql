ALTER TABLE users ADD COLUMN phone_number TEXT;
ALTER TABLE users ADD COLUMN emergency_contact_phone_number TEXT;

DROP VIEW IF EXISTS user_details_with_role;
CREATE VIEW user_details_with_role AS
SELECT u.id,
       u.email,
       u.name,
       u.nickname,
       u.phone_number,
       u.emergency_contact_phone_number,
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

DROP TRIGGER IF EXISTS anonymize_deleted_user;
CREATE TRIGGER anonymize_deleted_user
    AFTER UPDATE OF is_deleted ON users
    WHEN NEW.is_deleted = TRUE
BEGIN
    -- Anonymize user data
    UPDATE users
    SET email = 'deleted_' || NEW.id || '@archived.acspa.ro',
        name = 'Deleted User',
        nickname = NULL,
        phone_number = NULL,
        emergency_contact_phone_number = NULL,
        password_hash = 'REMOVED',
        has_key = FALSE
    WHERE id = NEW.id;

    -- Cancel all future active reservations for this user
    UPDATE reservations
    SET cancelled = TRUE
    WHERE user_id = NEW.id
      AND cancelled = FALSE
      AND (date > date('now') OR (date = date('now') AND hour >= strftime('%H', 'now')));
END;
