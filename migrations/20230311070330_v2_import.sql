ALTER TABLE telegram 
ADD user_id BLOB(16);

ALTER TABLE peers 
ADD id BLOB(16);

UPDATE telegram SET user_id = randomblob(16);
UPDATE peers SET id = randomblob(16);

INSERT INTO users (id)
    SELECT user_id
        FROM   telegram;

INSERT INTO roles (id, name) VALUES(X'22129C89706949CE9F4AF85004A7F230', "admin");

INSERT INTO user_roles (user_id, role_id) 
    SELECT telegram.user_id, X'22129C89706949CE9F4AF85004A7F230' 
        FROM telegram WHERE is_admin;

INSERT INTO integrations (user_id, telegram_id)
    SELECT user_id, id
        FROM   telegram;

INSERT INTO keys (user_id, key, name)
    SELECT first_user.id, public_key, peers.ip
        FROM peers
            JOIN (
                SELECT id FROM users LIMIT 1
            ) as first_user;

INSERT INTO configs (id, user_id, key, name)
    SELECT peers.id, first_user.id, public_key, peers.ip 
        FROM peers
            JOIN (
                SELECT id FROM users LIMIT 1
            ) as first_user;

INSERT INTO ips (config_id, addr)
    SELECT peers.id, peers.ip FROM peers;

DROP TABLE telegram;
DROP TABLE stats;
DROP TABLE settings;
DROP TABLE requests;
DROP TABLE peers;
