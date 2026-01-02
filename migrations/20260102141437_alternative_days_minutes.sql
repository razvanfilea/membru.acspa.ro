ALTER TABLE alternative_days
    ADD COLUMN
        slots_start_minute TINYINT CHECK (slots_start_minute > 0 AND slots_start_minute < 60);
