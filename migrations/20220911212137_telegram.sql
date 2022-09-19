-- Add migration script here
CREATE TABLE telegram (
    id INT8 NOT NULL PRIMARY KEY,
    associated_ip INT4,
    is_admin TINYINT NOT NULL
);
