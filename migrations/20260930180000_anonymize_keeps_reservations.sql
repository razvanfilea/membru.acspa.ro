-- Upcoming reservations of deleted users are removed by the app, which also promotes the waiting list
-- and records who deleted the user
DROP TRIGGER IF EXISTS anonymize_deleted_user;
CREATE TRIGGER anonymize_deleted_user
    AFTER UPDATE OF is_deleted ON users
    WHEN NEW.is_deleted = TRUE
BEGIN
    UPDATE users
    SET email = 'deleted_' || NEW.id || '@archived.acspa.ro',
        name = 'Deleted User',
        nickname = NULL,
        phone_number = NULL,
        emergency_contact_phone_number = NULL,
        password_hash = 'REMOVED',
        has_key = FALSE
    WHERE id = NEW.id;
END;
