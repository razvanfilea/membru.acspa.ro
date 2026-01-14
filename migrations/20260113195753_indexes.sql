CREATE INDEX idx_reservations_location_date_guest
    ON reservations (location, date, as_guest, created_at);

CREATE INDEX idx_payment_breaks_end_date ON payment_breaks (end_date);

DROP INDEX idx_payments_user_id;
CREATE INDEX idx_payments_user_date ON payments (user_id, payment_date DESC);

ALTER TABLE global_vars
    ADD check_payments BOOLEAN NOT NULL DEFAULT FALSE CHECK ( check_payments IN (FALSE, TRUE) );

DROP TRIGGER anonymize_deleted_user;

CREATE TRIGGER anonymize_deleted_user
    AFTER UPDATE OF is_deleted ON users
    WHEN NEW.is_deleted = TRUE
BEGIN
    -- Anonymize user data
    UPDATE users
    SET email = 'deleted_' || NEW.id || '@archived.acspa.ro',
        name = 'Deleted User',
        password_hash = 'REMOVED',
        has_key = FALSE
    WHERE id = NEW.id;

    -- Cancel all future active reservations for this user
    UPDATE reservations
    SET cancelled = TRUE
    WHERE user_id = NEW.id
      AND cancelled = FALSE
      AND (date > date('now') OR (date = date('now') AND hour >= strftime('%H', 'now')));
END;
