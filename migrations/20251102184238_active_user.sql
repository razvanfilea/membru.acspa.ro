PRAGMA defer_foreign_keys = ON;

CREATE TABLE users_new
(
    id            INTEGER NOT NULL PRIMARY KEY,
    email         TEXT    NOT NULL UNIQUE,
    name          TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE CHECK (is_active IN (FALSE, TRUE)),
    role_id       INTEGER NOT NULL REFERENCES user_roles (id),
    has_key       BOOLEAN NOT NULL DEFAULT FALSE CHECK (has_key IN (FALSE, TRUE)),
    birthday      DATE    NOT NULL,
    member_since  DATE    NOT NULL,
    received_gift DATE
);

INSERT INTO users_new
(id,
 email,
 name,
 password_hash,
 role_id,
 has_key,
 birthday,
 member_since,
 received_gift)
SELECT id,
       email,
       name,
       password_hash,
       role_id,
       has_key,
       birthday,
       member_since,
       received_gift
FROM users;

DROP VIEW users_with_role;
DROP TABLE users;

ALTER TABLE users_new
    RENAME TO users;

CREATE VIEW users_with_role AS
SELECT u.*,
       r.name AS role,
       r.admin_panel_access
FROM users u
         JOIN user_roles r ON u.role_id = r.id;

PRAGMA defer_foreign_keys = OFF;
