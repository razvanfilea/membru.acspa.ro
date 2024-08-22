INSERT INTO global_vars
VALUES (0, FALSE, '123456', '');

INSERT INTO user_roles
VALUES (1, 'Admin', 10, 10, NULL, TRUE),
       (2, 'Fondator', 3, 3, NULL, TRUE);

INSERT INTO locations
VALUES (1, 'Gară', 8, 4, 18, 2, 2, 10, 3, 4);

INSERT INTO users (id, email, name, password_hash, role_id, has_key)
VALUES (0, 'razvan.filea@gmail.com', 'Test Administrator',
        '$argon2id$v=19$m=19456,t=2,p=1$r7gp/pJoX038RwBEe8IzzQ$9L3znCPi4Va1ENFjxU4mIUkqsJdDHW2BiO81aPpfjiM', 1, TRUE);
