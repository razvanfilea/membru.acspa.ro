CREATE TABLE alternative_days_type
(
    type TEXT NOT NULL PRIMARY KEY
);

INSERT INTO alternative_days_type
VALUES ('holiday'),
       ('turneu');

ALTER TABLE locations
    DROP alt_slots_start_hour;
ALTER TABLE locations
    DROP alt_slot_duration;
ALTER TABLE locations
    DROP alt_slots_per_day;

CREATE TABLE alternative_days
(
    date             DATE     NOT NULL PRIMARY KEY,
    description      TEXT,
    type             TEXT     NOT NULL,

    slots_start_hour TINYINT  NOT NULL
        CHECK ( slots_start_hour > 0 AND slots_start_hour < 24 ),
    slot_duration    TINYINT  NOT NULL
        CHECK ( slot_duration > 0 AND slot_duration < 12 ),
    slots_per_day    TINYINT  NOT NULL
        CHECK ( slots_per_day > 0 AND slots_per_day < 12 ),

    created_at       DATETIME NOT NULL DEFAULT (datetime(CURRENT_TIMESTAMP, 'localtime')),

    FOREIGN KEY (type) REFERENCES alternative_days_type (type)
);

INSERT INTO alternative_days (date, description, type, slots_start_hour, slot_duration, slots_per_day, created_at)
SELECT date, description, 'holiday', 10, 3, 4, created_at FROM free_days;
DROP TABLE free_days;
