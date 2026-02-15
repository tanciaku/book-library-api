# Book Library API

A simple REST API for managing a personal book library, built with Rust and Axum.

## Features

- CRUD operations for books
- In-memory storage
- Track book availability

## Quick Start

```bash
cargo run
```

The server will start on `http://localhost:3000`

## API Endpoints

### Books

- `GET /health` - Health check
- `GET /books` - List all books
- `POST /books` - Add a new book
- `GET /books/{id}` - Get a book by ID
- `PUT /books/{id}` - Update a book
- `DELETE /books/{id}` - Delete a book

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

## Notes

⚠️ Uses in-memory storage - data is lost when the server stops.

## License

MIT
