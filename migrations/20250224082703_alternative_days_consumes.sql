ALTER TABLE alternative_days
    ADD COLUMN
        consumes_reservation BOOLEAN NOT NULL CHECK (consumes_reservation IN (FALSE, TRUE)) DEFAULT TRUE;
