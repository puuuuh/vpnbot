-- Add migration script here
CREATE TABLE requests (
    id BLOB NOT NULL PRIMARY KEY,
    telegram_id INT8,
    status INT4 NOT NULL,
    FOREIGN KEY(telegram_id) REFERENCES telegram(id)
);
