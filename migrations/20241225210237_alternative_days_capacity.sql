ALTER TABLE alternative_days
    ADD COLUMN
        slot_capacity TINYINT CHECK ( slot_capacity >= 0 );
