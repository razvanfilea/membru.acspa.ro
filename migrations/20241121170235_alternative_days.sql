ALTER TABLE locations
    DROP alt_slots_start_hour;
ALTER TABLE locations
    DROP alt_slot_duration;
ALTER TABLE locations
    DROP alt_slots_per_day;

ALTER TABLE free_days
    ADD
        slots_start_hour TINYINT NOT NULL DEFAULT 10
            CHECK ( slots_start_hour > 0 AND slots_start_hour < 24 );

ALTER TABLE free_days
    ADD
        slot_duration TINYINT NOT NULL DEFAULT 3
            CHECK ( slot_duration > 0 AND slot_duration < 12 );

ALTER TABLE free_days
    ADD slots_per_day TINYINT NOT NULL DEFAULT 4
        CHECK ( slots_per_day > 0 AND slots_per_day < 12 );

ALTER TABLE free_days
    ADD free_day BOOLEAN NOT NULL DEFAULT TRUE CHECK (free_day IN (FALSE, TRUE));


ALTER TABLE free_days
    RENAME TO alternative_days;
