CREATE TABLE user_roles
(
    id                     INTEGER NOT NULL PRIMARY KEY,
    name                   TEXT    NOT NULL UNIQUE,
    max_reservations       TINYINT NOT NULL CHECK (max_reservations >= 0),
    max_guest_reservations TINYINT NOT NULL CHECK (max_guest_reservations >= 0),
    admin_panel_access     BOOLEAN NOT NULL CHECK (admin_panel_access IN (FALSE, TRUE))
);

CREATE TABLE users
(
    id            INTEGER NOT NULL PRIMARY KEY,
    email         TEXT    NOT NULL UNIQUE,
    name          TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,

    role_id       INTEGER NOT NULL,
    has_key       BOOLEAN NOT NULL DEFAULT FALSE CHECK (has_key IN (FALSE, TRUE)),

    FOREIGN KEY (role_id) REFERENCES user_roles (id)
);

CREATE TABLE locations
(
    id                   INTEGER NOT NULL PRIMARY KEY,
    name                 TEXT    NOT NULL,
    slot_capacity        TINYINT NOT NULL CHECK ( slot_capacity > 0 ),

    slots_start_hour     TINYINT NOT NULL CHECK ( slots_start_hour > 0 AND slots_start_hour < 24 ),
    slot_duration        TINYINT NOT NULL CHECK ( slot_duration > 0 AND slot_duration < 12 ),
    slots_per_day        TINYINT NOT NULL
        CHECK ( slots_per_day > 0 AND slots_start_hour + slots_per_day * slot_duration < 24 ),

    alt_slots_start_hour TINYINT
        CHECK ( alt_slots_start_hour > 0 AND alt_slots_start_hour < 24 ),
    alt_slot_duration    TINYINT
        CHECK ( alt_slot_duration > 0 AND alt_slot_duration < 12 ),
    alt_slots_per_day    TINYINT
        CHECK ( alt_slots_per_day > 0 AND alt_slots_per_day < 12 )
);

CREATE TABLE reservations
(
    user_id    INTEGER  NOT NULL,
    date       DATE     NOT NULL,
    hour       TINYINT  NOT NULL,
    location   INTEGER  NOT NULL,
    cancelled  BOOLEAN  NOT NULL DEFAULT FALSE CHECK (cancelled IN (FALSE, TRUE)),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (user_id, date, hour, location),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (id)
);

CREATE TABLE reservations_restrictions
(
    date       DATE     NOT NULL PRIMARY KEY,
    hour       TINYINT  NOT NULL,
    message    TEXT     NOT NULL,
    location   TEXT     NOT NULL,

    created_by INTEGER  NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (created_by) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (name)
);

CREATE TABLE free_days
(
    date        DATE     NOT NULL PRIMARY KEY,
    description TEXT     NOT NULL,

    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE special_guests
(
    name       TEXT     NOT NULL,
    location   INTEGER  NOT NULL,
    date       DATE     NOT NULL,
    hour       TINYINT  NOT NULL,

    created_by INTEGER  NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (name, location, date, hour),
    FOREIGN KEY (created_by) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (id)
);

CREATE INDEX special_guests_created_by ON special_guests (created_by);

CREATE TABLE guests
(
    location   INTEGER  NOT NULL,
    date       DATE     NOT NULL,
    hour       TINYINT  NOT NULL,

    created_by INTEGER  NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (location, date, hour, created_by),
    FOREIGN KEY (created_by) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (id)
);

CREATE INDEX guests_created_by ON guests (created_by);

CREATE TABLE global_vars
(
    id               INTEGER NOT NULL PRIMARY KEY CHECK (id = 0),
    in_maintenance   BOOLEAN NOT NULL CHECK (in_maintenance IN (FALSE, TRUE)),
    entrance_code    TEXT    NOT NULL,
    reminder_message TEXT    NOT NULL
);

CREATE VIEW users_with_role AS
SELECT u.id,
       u.email,
       u.name,
       u.password_hash,
       u.has_key,
       r.name AS role,
       r.admin_panel_access
FROM users u
         INNER JOIN user_roles r ON u.role_id = r.id;
