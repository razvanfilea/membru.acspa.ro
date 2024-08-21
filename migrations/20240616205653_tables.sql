CREATE TABLE user_roles
(
    id                 INTEGER NOT NULL PRIMARY KEY,
    name               TEXT    NOT NULL UNIQUE,
    reservations       TINYINT NOT NULL CHECK (reservations >= 0),
    guest_reservations TINYINT NOT NULL CHECK (guest_reservations >= 0),
    color              TEXT,
    admin_panel_access BOOLEAN NOT NULL DEFAULT FALSE CHECK (admin_panel_access IN (FALSE, TRUE))
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
    user_id     INTEGER  NOT NULL,
    date        DATE     NOT NULL,
    hour        TINYINT  NOT NULL,
    location    INTEGER  NOT NULL,

    created_for TEXT,
    as_guest    BOOLEAN  NOT NULL DEFAULT FALSE CHECK (as_guest IN (FALSE, TRUE)),

    cancelled   BOOLEAN  NOT NULL DEFAULT FALSE CHECK (cancelled IN (FALSE, TRUE)),
    in_waiting  BOOLEAN  NOT NULL DEFAULT FALSE CHECK (in_waiting IN (FALSE, TRUE)),

    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (user_id, date, hour, location, created_for),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (id)
);

CREATE TABLE reservations_restrictions
(
    date       DATE     NOT NULL,
    hour       TINYINT,
    location   INTEGER  NOT NULL,
    message    TEXT     NOT NULL,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (date, hour, location),
    FOREIGN KEY (location) REFERENCES locations (id)
);

CREATE TABLE free_days
(
    date        DATE     NOT NULL PRIMARY KEY,
    description TEXT,

    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE global_vars
(
    id               INTEGER NOT NULL PRIMARY KEY CHECK (id = 0),
    in_maintenance   BOOLEAN NOT NULL CHECK (in_maintenance IN (FALSE, TRUE)),
    entrance_code    TEXT    NOT NULL,
    homepage_message TEXT    NOT NULL
);

CREATE VIEW users_with_role AS
SELECT u.id,
       u.email,
       u.name,
       u.password_hash,
       u.has_key,
       r.name  AS role,
       r.color AS role_color,
       r.admin_panel_access
FROM users u
         INNER JOIN user_roles r ON u.role_id = r.id;
