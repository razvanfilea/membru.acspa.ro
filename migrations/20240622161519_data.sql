insert into user_roles
VALUES ('Fondator'),
       ('Antrenor'),
       ('Cotizant 1s'),
       ('Cotizant 2s'),
       ('Inactiv'),
       ('Membru');

insert into users (email, name, password_hash, role, has_key)
VALUES ('razvan.filea@gmail.com', 'Razvan Filea', '$argon2id$v=19$m=19456,t=2,p=1$r7gp/pJoX038RwBEe8IzzQ$9L3znCPi4Va1ENFjxU4mIUkqsJdDHW2BiO81aPpfjiM', 'Fondator', TRUE);

insert into locations (name, slot_capacity, slots_start_hour, slot_duration, slots_per_day)
VALUES ('Gara', 8, 18, 2, 2);
