INSERT INTO global_vars VALUES (0, FALSE, '123456', '');

INSERT INTO user_roles
VALUES (1, 'Admin', 100, 100, TRUE),
       (2, 'Fondator', 3, 3, TRUE);

INSERT INTO locations (id, name, slot_capacity, slots_start_hour, slot_duration, slots_per_day, alt_slots_start_hour,
                       alt_slot_duration, alt_slots_per_day)
VALUES (1, 'Gara', 8, 18, 2, 2, 10, 3, 4);

-- ('Inactiv 2'), -- Nu este achitata cotizația: nu poate sa facă rezervare (mesaj: nu a plătit)
-- ('Inactiv 1'), -- Nu este achitata cotizația de 1 lună: drepturi de Cotizant 1s fara invitat antrenamente
-- ('Invitat'), -- Poate face o rezervare ca invitat
-- ('Copil'), -- La fel ca și invitat
-- ('Antrenor'), -- Poate face 1 rezervări pe săptămână
-- ('Cotizant 1s'), -- Poate face 1 rezervări pe săptămână și rezervare ca invitat
-- ('Cotizant 2s'), -- Poate face 2 rezervări pe săptămână și rezervare ca invitat
-- ('Fondator'), -- Poate face 3 rezervări pe săptămână și rezervare ca invitat, access la panou
-- ('Admin'); -- Face ce vrea

insert into users (email, name, password_hash, role_id, has_key)
VALUES ('razvan.filea@gmail.com', 'Razvan Filea',
        '$argon2id$v=19$m=19456,t=2,p=1$r7gp/pJoX038RwBEe8IzzQ$9L3znCPi4Va1ENFjxU4mIUkqsJdDHW2BiO81aPpfjiM', 1, TRUE);
