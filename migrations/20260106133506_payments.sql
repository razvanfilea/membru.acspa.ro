CREATE TABLE payments
(
    id           INTEGER  NOT NULL PRIMARY KEY,
    user_id      INTEGER  NOT NULL,
    amount       INTEGER  NOT NULL,
    payment_date DATE     NOT NULL,
    notes        TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by   INTEGER  NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
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

    -- Convention: both dates are the 1st of the starting month.
    start_date DATE     NOT NULL,
    end_date   DATE     NOT NULL CHECK (end_date >= start_date),

    reason     TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER  NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE RESTRICT,
    CHECK (end_date >= start_date)
);
