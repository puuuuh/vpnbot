-- Add migration script here
CREATE TABLE peers (
    ip INT4 NOT NULL PRIMARY KEY,
    public_key BLOB NOT NULL UNIQUE
);

CREATE TABLE settings (
    ip INT4 NOT NULL PRIMARY KEY,
    double_vpn TINYINT NOT NULL
);
