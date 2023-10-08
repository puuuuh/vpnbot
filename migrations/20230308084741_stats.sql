-- Add migration script here
CREATE TABLE stats (
    public_key BLOB NOT NULL PRIMARY KEY,
    transmitted INT8 NOT NULL,
    received INT8 NOT NULL
);
