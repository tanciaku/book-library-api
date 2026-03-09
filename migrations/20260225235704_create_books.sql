CREATE TABLE books (
    id        BIGSERIAL PRIMARY KEY,
    title     TEXT      NOT NULL,
    author    TEXT      NOT NULL,
    year      BIGINT    NOT NULL,
    isbn      TEXT      NOT NULL,
    available BOOLEAN   NOT NULL
);