CREATE TABLE IF NOT EXISTS borrowings (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id       INTEGER NOT NULL REFERENCES books(id),
    borrower_name TEXT NOT NULL,
    borrowed_at   TEXT NOT NULL,
    due_date      TEXT NOT NULL,
    returned_at   TEXT
);