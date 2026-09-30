ALTER TABLE reservations ADD COLUMN cancelled_by INTEGER REFERENCES users (id);

CREATE TABLE reservation_history
(
    id               INTEGER PRIMARY KEY,
    event            TEXT     NOT NULL,
    user_id          INTEGER  NOT NULL,
    date             DATE     NOT NULL,
    hour             TINYINT  NOT NULL,
    old_hour         TINYINT,
    location         INTEGER  NOT NULL,
    created_for      TEXT,
    as_guest         BOOLEAN  NOT NULL,
    in_waiting       BOOLEAN  NOT NULL,
    waiting_position INTEGER,
    -- NULL means the change was made by the system
    changed_by       INTEGER,
    changed_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_reservation_history_date_user ON reservation_history (date, user_id);
CREATE INDEX idx_reservation_history_changed_at ON reservation_history (changed_at);
CREATE INDEX idx_reservation_history_user_changed_at ON reservation_history (user_id, changed_at);

CREATE TRIGGER reservation_history_insert
    AFTER INSERT ON reservations
BEGIN
    INSERT INTO reservation_history (event, user_id, date, hour, location, created_for, as_guest, in_waiting,
                                     waiting_position, changed_by)
    VALUES ('created', NEW.user_id, NEW.date, NEW.hour, NEW.location, NEW.created_for, NEW.as_guest,
            NEW.in_waiting,
            CASE
                WHEN NEW.in_waiting THEN 1 + (SELECT count(*)
                                              FROM reservations r
                                              WHERE r.date = NEW.date
                                                AND r.hour = NEW.hour
                                                AND r.location = NEW.location
                                                AND r.in_waiting
                                                AND NOT r.cancelled
                                                AND (r.as_guest, r.created_at, r.rowid) <
                                                    (NEW.as_guest, NEW.created_at, NEW.rowid))
                END,
            NEW.user_id);
END;

CREATE TRIGGER reservation_history_update
    AFTER UPDATE ON reservations
    -- Skip updates that only stamp cancelled_by right before a delete
    WHEN NOT (OLD.cancelled_by IS NOT NEW.cancelled_by
        AND OLD.user_id = NEW.user_id
        AND OLD.date = NEW.date
        AND OLD.hour = NEW.hour
        AND OLD.location = NEW.location
        AND OLD.created_for IS NEW.created_for
        AND OLD.as_guest = NEW.as_guest
        AND OLD.cancelled = NEW.cancelled
        AND OLD.in_waiting = NEW.in_waiting)
BEGIN
    INSERT INTO reservation_history (event, user_id, date, hour, old_hour, location, created_for, as_guest,
                                     in_waiting, waiting_position, changed_by)
    VALUES (CASE
                WHEN NOT OLD.cancelled AND NEW.cancelled THEN 'cancelled'
                WHEN OLD.in_waiting AND NOT NEW.in_waiting THEN 'promoted'
                WHEN NOT OLD.in_waiting AND NEW.in_waiting THEN 'moved_to_waiting'
                WHEN OLD.hour != NEW.hour THEN 'hour_changed'
                ELSE 'updated'
                END,
            NEW.user_id, NEW.date, NEW.hour,
            CASE WHEN OLD.hour != NEW.hour THEN OLD.hour END,
            NEW.location, NEW.created_for, NEW.as_guest, NEW.in_waiting,
            CASE
                WHEN NEW.in_waiting AND NOT NEW.cancelled THEN 1 + (SELECT count(*)
                                                                   FROM reservations r
                                                                   WHERE r.date = NEW.date
                                                                     AND r.hour = NEW.hour
                                                                     AND r.location = NEW.location
                                                                     AND r.in_waiting
                                                                     AND NOT r.cancelled
                                                                     AND (r.as_guest, r.created_at, r.rowid) <
                                                                         (NEW.as_guest, NEW.created_at, NEW.rowid))
                END,
            CASE WHEN NOT OLD.cancelled AND NEW.cancelled THEN NEW.cancelled_by END);
END;

CREATE TRIGGER reservation_history_delete
    AFTER DELETE ON reservations
    -- A cancelled row's story ends at its cancellation, deleting it later is housekeeping
    WHEN NOT OLD.cancelled
BEGIN
    INSERT INTO reservation_history (event, user_id, date, hour, location, created_for, as_guest, in_waiting,
                                     changed_by)
    VALUES (CASE
                -- Still waiting when its slot started, so it never got a spot
                WHEN OLD.in_waiting AND (OLD.date < date('now', 'localtime') OR
                                         (OLD.date = date('now', 'localtime') AND
                                          OLD.hour <= CAST(strftime('%H', 'now', 'localtime') AS INTEGER)))
                    THEN 'expired'
                ELSE 'deleted'
                END,
            OLD.user_id, OLD.date, OLD.hour, OLD.location, OLD.created_for, OLD.as_guest, OLD.in_waiting,
            -- Set only for named guests, stamped right before the delete
            OLD.cancelled_by);
END;
