CREATE TABLE users (id BLOB(16) PRIMARY KEY NOT NULL);

CREATE TABLE roles (id BLOB(16) PRIMARY KEY NOT NULL, name TEXT);

CREATE TABLE user_roles (
    user_id BLOB(16) NOT NULL, 
    role_id BLOB(16) NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(role_id) REFERENCES roles(id)
);

CREATE TABLE keys (
    key BLOB(32) PRIMARY KEY NOT NULL,
    user_id BLOB(16) NOT NULL,
    name TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE configs (
    id BLOB(16) PRIMARY KEY NOT NULL,
    user_id BLOB(16) NOT NULL,
    key BLOB(32) NOT NULL,
    name TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(key) REFERENCES keys(key)
);

CREATE TABLE ips (
    config_id BLOB(16) NOT NULL,
    addr INT(4) NOT NULL UNIQUE,
    FOREIGN KEY(config_id) REFERENCES configs(id)
);

CREATE TABLE integrations (
    user_id BLOB(16) NOT NULL,
    telegram_id INTEGER8 UNIQUE,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE stats_v2 (
    key BLOB(32) PRIMARY KEY NOT NULL,
    tx INTEGER NOT NULL,
    rx INTEGER NOT NULL,
    FOREIGN KEY(key) REFERENCES keys(key)
);

CREATE UNIQUE INDEX integrations_tgid_index ON integrations (telegram_id);
