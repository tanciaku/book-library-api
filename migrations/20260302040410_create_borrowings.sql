CREATE TABLE IF NOT EXISTS borrowings (
    id            BIGSERIAL   PRIMARY KEY,
    book_id       BIGINT      NOT NULL REFERENCES books(id),
    borrower_name TEXT        NOT NULL,
    borrowed_at   TIMESTAMPTZ NOT NULL,
    due_date      TIMESTAMPTZ NOT NULL,
    returned_at   TIMESTAMPTZ
);