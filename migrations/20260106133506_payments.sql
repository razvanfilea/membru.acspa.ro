CREATE TABLE payments
(
    id           INTEGER  NOT NULL PRIMARY KEY,
    user_id      INTEGER  NOT NULL,
    amount       INTEGER  NOT NULL,
    payment_date DATE     NOT NULL,
    notes        TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by   INTEGER  NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE RESTRICT
);

CREATE TABLE payment_allocations
(
    payment_id INTEGER NOT NULL,
    year       INTEGER NOT NULL,
    month      TINYINT NOT NULL CHECK (month >= 1 AND month <= 12),

    PRIMARY KEY (payment_id, year, month),
    FOREIGN KEY (payment_id) REFERENCES payments (id) ON DELETE CASCADE
);

CREATE TABLE payment_breaks
(
    id         INTEGER  NOT NULL PRIMARY KEY,
    user_id    INTEGER  NOT NULL,

    -- Enforce that dates are the 1st of the month
    start_date DATE     NOT NULL CHECK (strftime('%d', start_date) = '01'),
    end_date   DATE     NOT NULL CHECK (strftime('%d', end_date) = '01'),

    reason     TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER  NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE RESTRICT,

    CHECK (end_date >= start_date)
);

CREATE TRIGGER prevent_duplicate_payment_allocation
    BEFORE INSERT
    ON payment_allocations
BEGIN
    SELECT RAISE(ABORT, 'This month is already covered by another payment for this user.')
    WHERE EXISTS (SELECT 1
                  FROM payment_allocations pa
                           JOIN payments p_existing ON pa.payment_id = p_existing.id
                           JOIN payments p_new ON p_new.id = NEW.payment_id
                  WHERE
                    -- Match the user
                      p_existing.user_id = p_new.user_id
                    -- Match the period (Year/Month)
                    AND pa.year = NEW.year
                    AND pa.month = NEW.month);
END;

CREATE INDEX idx_payments_user_id ON payments (user_id);
CREATE INDEX idx_payment_breaks_user_id ON payment_breaks (user_id);
