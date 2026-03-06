# Book Library API

A simple REST API for managing a personal book library, built with Rust and Axum.

## Features

- CRUD operations for books
- SQLite persistence via sqlx
- Track book availability
- Borrow and return books with configurable loan periods
- List overdue borrowings

## Quick Start

1. Create a `.env` file (or export the variable) with your database URL:

```bash
echo "DATABASE_URL=sqlite:books.db" > .env
```

2. Run database migrations:

```bash
cargo sqlx migrate run
```

3. Start the server:

```bash
cargo run
```

The server will start on `http://localhost:3000`

## API Endpoints

### Books

- `GET /health` - Health check
- `GET /books` - List all books (with optional filters and pagination)
- `POST /books` - Add a new book
- `GET /books/{id}` - Get a book by ID
- `PUT /books/{id}` - Update a book
- `DELETE /books/{id}` - Delete a book

### Borrowings

- `POST /books/{id}/borrow` - Borrow a book
- `POST /books/{id}/return` - Return a borrowed book
- `GET /borrowings/overdue` - List all overdue borrowings

### Example Requests

**Add a book:**
```bash
curl -X POST http://localhost:3000/books \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Clean Code",
    "author": "Robert C. Martin",
    "year": 2008,
    "isbn": "978-0132350884"
  }'
```

**List all books:**
```bash
curl http://localhost:3000/books
```

**Filter books:**
```bash
# Filter by availability
curl http://localhost:3000/books?available=true

# Filter by author (case-insensitive search)
curl http://localhost:3000/books?author=martin

# Filter by publication year
curl http://localhost:3000/books?year=2008

# Combine multiple filters
curl "http://localhost:3000/books?available=true&author=martin&year=2008"
```

**Paginate books:**
```bash
# Get the second page with 5 books per page
curl "http://localhost:3000/books?page=2&limit=5"

# Combine pagination with filters
curl "http://localhost:3000/books?available=true&page=1&limit=20"
```

The response includes a `pagination` metadata object alongside the `data` array:
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total_items": 42,
    "total_pages": 5
  }
}
```

> `page` defaults to `1` and `limit` defaults to `10` (max `100`).

**Update book availability:**
```bash
curl -X PUT http://localhost:3000/books/1 \
  -H "Content-Type: application/json" \
  -d '{"available": false}'
```

**Delete a book:**
```bash
curl -X DELETE http://localhost:3000/books/1
```

**Borrow a book:**
```bash
curl -X POST http://localhost:3000/books/1/borrow \
  -H "Content-Type: application/json" \
  -d '{"borrower_name": "Alice", "days": 7}'
```

> `days` is optional and defaults to `14`.

Returns `201 Created` with the borrowing record, or `404` if the book doesn't exist, or `409 Conflict` if the book is already borrowed.

**Return a book:**
```bash
curl -X POST http://localhost:3000/books/1/return
```

Returns `200 OK`, or `400 Bad Request` if the book is not currently borrowed.

**List overdue borrowings:**
```bash
curl http://localhost:3000/borrowings/overdue
```

Returns all borrowings whose `due_date` has passed and that have not yet been returned:
```json
[
  {
    "borrowing_id": 3,
    "book_id": 1,
    "book_title": "Clean Code",
    "book_author": "Robert C. Martin",
    "borrower_name": "Alice",
    "borrowed_at": "2024-01-01T00:00:00+00:00",
    "due_date": "2024-01-15T00:00:00+00:00"
  }
]
```

## Data Model

```json
{
  "id": 1,
  "title": "Book Title",
  "author": "Author Name",
  "year": 2024,
  "isbn": "978-1234567890",
  "available": true
}
```

## Validation

When adding a new book, the following validations are enforced:

- **Title**: Must not be empty
- **Author**: Must not be empty
- **Year**: Must be between 1000 and the current year
- **ISBN**: Must be a valid ISBN-13 format (13 digits, hyphens allowed)

Invalid requests will return `400 Bad Request` with an error message.

## Testing

The project includes a comprehensive test suite covering all endpoints with both unit and integration tests.

Test coverage includes:
- All CRUD operations and their expected status codes
- Input validation (empty fields, invalid ISBN, future year)
- Filtering by author (case-insensitive), year, and availability
- Pagination correctness, limit capping, and out-of-bounds pages
- End-to-end integration flows (create → update → get, create → delete → 404, etc.)
- Borrow/return lifecycle (201 on borrow, 409 on double-borrow, 200 on return, 400 on bad return)
- Overdue list filtering (excludes returned and future-due borrowings)
- End-to-end borrow → return flow verifying `available` flag transitions

## Notes

- Data is persisted in `books.db` (SQLite). The file is created automatically on first run.
- SQLite foreign key enforcement is enabled on all connections.
- Tests use an isolated in-memory SQLite database and run migrations automatically.

## License

MIT
