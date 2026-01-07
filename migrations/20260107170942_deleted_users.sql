ALTER TABLE users
    ADD is_deleted BOOLEAN NOT NULL DEFAULT FALSE CHECK ( is_deleted IN (FALSE, TRUE) );

DROP VIEW users_with_role;

CREATE VIEW users_with_role AS
SELECT u.*,
       r.name AS role,
       r.admin_panel_access
FROM users u
         INNER JOIN user_roles r ON u.role_id = r.id
WHERE is_deleted = FALSE;

CREATE TRIGGER anonymize_deleted_user
    AFTER UPDATE OF is_deleted ON users
    WHEN NEW.is_deleted = TRUE
BEGIN
    UPDATE users
    SET email = 'deleted_' || NEW.id || '@archived.acspa.ro',
        name = 'Deleted User',
        password_hash = 'REMOVED',
        has_key = FALSE
    WHERE id = NEW.id;
END;
