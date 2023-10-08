-- Add migration script here
CREATE TABLE integration (
    ip INT4 NOT NULL,
    telegram_id INT8 NOT NULL,
    FOREIGN KEY(ip) REFERENCES peers(ip),
    FOREIGN KEY(telegram_id) REFERENCES telegram(id),
    UNIQUE(ip, telegram_id)
);
