CREATE DATABASE user_db;

\c user_db;

CREATE TABLE UserTable(
    id SERIAL PRIMARY KEY,
    username varchar(20) NOT NULL
);
