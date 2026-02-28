use axum::{Json, Router, extract::{Path, Query, State}, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use chrono::Datelike;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
    id: i64,
    title: String,
    author: String,
    year: i64,
    isbn: String,
    available: bool,
}

#[derive(Debug, Deserialize)]
struct AddBook {
    title: String,
    author: String,
    year: i64,
    isbn: String,
}

#[derive(Debug, Deserialize)]
struct UpdateBook {
    title: Option<String>,
    author: Option<String>,
    year: Option<i64>,
    isbn: Option<String>,
    available: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BookParams {
    available: Option<bool>,
    author: Option<String>,
    year: Option<i64>,
    page: Option<usize>,
    limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaginationMeta {
    page: usize,
    limit: usize,
    total_items: usize,
    total_pages: usize,
}

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect("sqlite:books.db")
        .await
        .unwrap();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/books", get(list_books).post(add_book))
        .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("\n Server running on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_books(
    State(pool): State<SqlitePool>,
    Query(params): Query<BookParams>
) -> Json<PaginatedResponse<Book>> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100);
    let offset = (page - 1) * limit;

    // (total_rows)
    let total_items = sqlx::query!(
        "SELECT COUNT(*) as count FROM books
         WHERE (? IS NULL OR available = ?)
         AND (? IS NULL OR LOWER(author) LIKE '%' || LOWER(?) || '%')
         AND (? IS NULL OR year = ?)",
        params.available,
        params.available,
        params.author,
        params.author,
        params.year,
        params.year,
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    as usize;

    let total_pages = (total_items + limit - 1) / limit;

    let limit_i64 = limit as i64;
    let offset_i64 = offset as i64;

    let rows = sqlx::query!(
        "SELECT * FROM books
         WHERE (? IS NULL OR available = ?)
         AND (? IS NULL OR LOWER(author) LIKE '%' || LOWER(?) || '%')
         AND (? IS NULL OR year = ?)
         LIMIT ? OFFSET ?",
        params.available,
        params.available,
        params.author,
        params.author,
        params.year,
        params.year,
        limit_i64,
        offset_i64,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let paginated_data: Vec<Book> = rows.into_iter().map(|r| Book {
        id: r.id,
        title: r.title,
        author: r.author,
        year: r.year,
        isbn: r.isbn,
        available: r.available != 0,
    }).collect();

    Json(PaginatedResponse {
        data: paginated_data,
        pagination: PaginationMeta {
            page,
            limit,
            total_items,
            total_pages,
        },
    })
}

async fn add_book(
    State(pool): State<SqlitePool>,
    Json(input): Json<AddBook>
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
    if !validate_book(&input) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid book data. Check title, author, year, and ISBN format.".to_string()
            })
        ));
    }
    
    let result = sqlx::query!(
        "INSERT INTO books (title, author, year, isbn, available) VALUES (?, ?, ?, ?, ?)",
        input.title,
        input.author,
        input.year,
        input.isbn,
        true,
    )
    .execute(&pool)
    .await
    .unwrap();

    let book = Book {
        id: result.last_insert_rowid(),
        title: input.title,
        author: input.author,
        year: input.year,
        isbn: input.isbn,
        available: true,
    };

    Ok((StatusCode::CREATED, Json(book)))
}

fn validate_book(book: &AddBook) -> bool {
    !book.title.is_empty() &&
    !book.author.is_empty() &&
    is_valid_year(book.year) &&
    is_valid_isbn(&book.isbn)
}

fn is_valid_year(year: i64) -> bool {
    let current_year = chrono::Utc::now().year() as i64;
    (1000..=current_year).contains(&year)
}

fn is_valid_isbn(isbn: &str) -> bool {
    let cleaned = isbn.replace("-", "");
    cleaned.len() == 13 && cleaned.chars().all(|c| c.is_numeric())
}

async fn get_book(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
    let row = sqlx::query!(
        "SELECT id, title, author, year, isbn, available FROM books WHERE id = ?",
        id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    match row {
        Some(r) => Ok((
            StatusCode::OK,
            Json(Book {
                id: r.id,
                title: r.title,
                author: r.author,
                year: r.year,
                isbn: r.isbn,
                available: r.available != 0,
            }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        ))),
    }
}

async fn update_book(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateBook>
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
    let result = sqlx::query!(
        "UPDATE books
         SET title     = COALESCE(?, title),
             author    = COALESCE(?, author),
             year      = COALESCE(?, year),
             isbn      = COALESCE(?, isbn),
             available = COALESCE(?, available)
         WHERE id = ?",
        input.title,
        input.author,
        input.year,
        input.isbn,
        input.available,
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        )))
    }

    let row = sqlx::query!(
        "SELECT id, title, author, year, isbn, available FROM books WHERE id = ?",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    Ok((
        StatusCode::OK,
        Json(Book {
            id: row.id,
            title: row.title,
            author: row.author,
            year: row.year,
            isbn: row.isbn,
            available: row.available != 0,
        })
    ))
}

async fn delete_book(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, ()), (StatusCode, Json<ErrorResponse>)> {
    let result = sqlx::query!(
        "DELETE FROM books WHERE id = ?",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    if result.rows_affected() == 0 {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        )))
    } else {
        Ok((StatusCode::NO_CONTENT, ()))
    }
}

#[cfg(test)]
mod tests;