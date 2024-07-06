insert into user_roles
VALUES ('Fondator'),
       ('Antrenor'),
       ('Cotizant 1s'),
       ('Cotizant 2s'),
       ('Inactiv'),
       ('Membru');

insert into users (email, name, password_hash, role, has_key)
VALUES ('razvan.filea@gmail.com', 'Razvan Filea', 'parola1234', 'Fondator', TRUE);

insert into locations (name, slot_capacity, slots_start_hour, slot_duration, slots_per_day)
VALUES ('Gara', 8, 18, 2, 2);
