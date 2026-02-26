CREATE TABLE books (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    title     TEXT    NOT NULL,
    author    TEXT    NOT NULL,
    year      INTEGER NOT NULL DEFAULT 0,
    isbn      TEXT    NOT NULL,
    available INTEGER NOT NULL DEFAULT 0
);