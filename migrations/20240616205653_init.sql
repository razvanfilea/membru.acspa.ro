CREATE TABLE user_roles
(
    role TEXT NOT NULL PRIMARY KEY
) STRICT;

CREATE TABLE users
(
    id            INTEGER NOT NULL PRIMARY KEY,
    email         TEXT    NOT NULL UNIQUE,
    name          TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,

    role          TEXT    NOT NULL,
    has_key       BOOLEAN NOT NULL CHECK (has_key IN (FALSE, TRUE)),

    FOREIGN KEY (role) REFERENCES user_roles (role)
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
    message    TEXT,
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
    guest_name TEXT     NOT NULL,
    location   INTEGER  NOT NULL,
    date       DATE     NOT NULL,
    hour       TINYINT  NOT NULL,

    created_by INTEGER  NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (guest_name, location, date, hour),
    FOREIGN KEY (created_by) REFERENCES users (id),
    FOREIGN KEY (location) REFERENCES locations (id)
);