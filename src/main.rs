use axum::{Json, Router, extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, routing::{get, post}};
use serde::{Deserialize, Serialize};
use chrono::{Datelike, DateTime, Utc};
use sqlx::PgPool;

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

enum AppError {
    Database(sqlx::Error),
    NotFound(i64),
    BadRequest,
    BookUnavailable(i64),
    NotBorrowed(i64),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e)
            )
                .into_response(),
            AppError::NotFound(id) => (
                StatusCode::NOT_FOUND,
                format!("Book with ID {} not found", id)
            )
                .into_response(),
            AppError::BadRequest => (
                StatusCode::BAD_REQUEST,
                "Invalid book data. Check title, author, year, and ISBN format.".to_string()
            )
                .into_response(),
            AppError::BookUnavailable(id) => (
                StatusCode::CONFLICT,
                format!("Book with ID {} is already borrowed", id)
            )
                .into_response(),
            AppError::NotBorrowed(id) => (
                StatusCode::BAD_REQUEST,
                format!("Book with ID {} is not borrowed", id)
            )
                .into_response(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Borrowing {
    id: i64,
    book_id: i64,
    borrower_name: String,
    borrowed_at: DateTime<Utc>,
    due_date: DateTime<Utc>,
    returned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct BorrowBook {
    borrower_name: String,
    days: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OverdueBorrowing {
    borrowing_id: i64,
    book_id: i64,
    book_title: String,
    book_author: String,
    borrower_name: String,
    borrowed_at: DateTime<Utc>,
    due_date: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&db_url).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/books", get(list_books).post(add_book))
        .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
        .route("/books/{id}/borrow", post(borrow_book))
        .route("/books/{id}/return", post(return_book))
        .route("/borrowings/overdue", get(list_overdue))
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
    State(pool): State<PgPool>,
    Query(params): Query<BookParams>
) -> Result<Json<PaginatedResponse<Book>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100);
    let offset = (page - 1) * limit;

    // (total_rows)
    let total_items = sqlx::query!(
        "SELECT COUNT(*) as count FROM books
         WHERE ($1::boolean IS NULL OR available = $1)
         AND ($2::text IS NULL OR LOWER(author) LIKE '%' || LOWER($2) || '%')
         AND ($3::bigint IS NULL OR year = $3)",
        params.available,
        params.author,
        params.year,
    )
    .fetch_one(&pool)
    .await?
    .count
    .unwrap_or(0) as usize;

    let total_pages = (total_items + limit - 1) / limit;

    let limit_i64 = limit as i64;
    let offset_i64 = offset as i64;

    let rows = sqlx::query!(
        "SELECT * FROM books
         WHERE ($1::boolean IS NULL OR available = $1)
         AND ($2::text IS NULL OR LOWER(author) LIKE '%' || LOWER($2) || '%')
         AND ($3::bigint IS NULL OR year = $3)
         LIMIT $4 OFFSET $5",
        params.available,
        params.author,
        params.year,
        limit_i64,
        offset_i64,
    )
    .fetch_all(&pool)
    .await?;

    let paginated_data: Vec<Book> = rows.into_iter().map(|r| Book {
        id: r.id,
        title: r.title,
        author: r.author,
        year: r.year,
        isbn: r.isbn,
        available: r.available,
    }).collect();

    Ok(Json(PaginatedResponse {
        data: paginated_data,
        pagination: PaginationMeta {
            page,
            limit,
            total_items,
            total_pages,
        },
    }))
}

async fn add_book(
    State(pool): State<PgPool>,
    Json(input): Json<AddBook>
) -> Result<(StatusCode, Json<Book>), AppError> {
    if !validate_book(&input) {
        return Err(AppError::BadRequest)
    }
    
    let row = sqlx::query!(
        "INSERT INTO books (title, author, year, isbn, available) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        input.title,
        input.author,
        input.year,
        input.isbn,
        true,
    )
    .fetch_one(&pool)
    .await?;

    let book = Book {
        id: row.id,
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
    State(pool): State<PgPool>,
    Path(id): Path<i64>
) -> Result<(StatusCode, Json<Book>), AppError> {
    let row = sqlx::query!(
        "SELECT id, title, author, year, isbn, available FROM books WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(r) => Ok((
            StatusCode::OK,
            Json(Book {
                id: r.id,
                title: r.title,
                author: r.author,
                year: r.year,
                isbn: r.isbn,
                available: r.available,
            }))),
        None => Err(AppError::NotFound(id)),
    }
}

async fn update_book(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateBook>
) -> Result<(StatusCode, Json<Book>), AppError> {
    let result = sqlx::query!(
        "UPDATE books
         SET title     = COALESCE($1, title),
             author    = COALESCE($2, author),
             year      = COALESCE($3, year),
             isbn      = COALESCE($4, isbn),
             available = COALESCE($5, available)
         WHERE id = $6",
        input.title,
        input.author,
        input.year,
        input.isbn,
        input.available,
        id
    )
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(id))
    }

    let row = sqlx::query!(
        "SELECT id, title, author, year, isbn, available FROM books WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(Book {
            id: row.id,
            title: row.title,
            author: row.author,
            year: row.year,
            isbn: row.isbn,
            available: row.available,
        })
    ))
}

async fn delete_book(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query!(
        "DELETE FROM books WHERE id = $1",
        id
    )
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(AppError::NotFound(id))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

async fn borrow_book(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(input): Json<BorrowBook>,
) -> Result<(StatusCode, Json<Borrowing>), AppError> {
    let book = sqlx::query!(
        "SELECT id, available FROM books WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?;

    let book = match book {
        Some(b) => b,
        None => return Err(AppError::NotFound(id)),
    };

    if !book.available {
        return Err(AppError::BookUnavailable(id));
    }

    let days = input.days.unwrap_or(14);
    let now = chrono::Utc::now();
    let borrowed_at: DateTime<Utc> = now;
    let due_date: DateTime<Utc> = now + chrono::Duration::days(days);

    let row = sqlx::query!(
        "INSERT INTO borrowings (book_id, borrower_name, borrowed_at, due_date) VALUES ($1, $2, $3, $4) RETURNING id",
        id,
        input.borrower_name,
        borrowed_at,
        due_date,
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!(
        "UPDATE books SET available = false WHERE id = $1",
        id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(Borrowing {
        id: row.id,
        book_id: id,
        borrower_name: input.borrower_name,
        borrowed_at,
        due_date,
        returned_at: None,
    })))
}

async fn return_book(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let borrowing = sqlx::query!(
        "SELECT id FROM borrowings WHERE book_id = $1 AND returned_at IS NULL",
        id
    )
    .fetch_optional(&pool)
    .await?;

    if borrowing.is_none() {
        return Err(AppError::NotBorrowed(id));
    }

    let returned_at: DateTime<Utc> = chrono::Utc::now();

    sqlx::query!(
        "UPDATE borrowings SET returned_at = $1 WHERE book_id = $2 AND returned_at IS NULL",
        returned_at,
        id
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "UPDATE books SET available = true WHERE id = $1",
        id
    )
    .execute(&pool)
    .await?;

    Ok(StatusCode::OK)
}

async fn list_overdue(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<OverdueBorrowing>>, AppError> {
    let now: DateTime<Utc> = chrono::Utc::now();

    let rows = sqlx::query!(
        "SELECT b.id as borrowing_id, b.book_id, bk.title as book_title,
                bk.author as book_author, b.borrower_name, b.borrowed_at, b.due_date
         FROM borrowings b
         JOIN books bk ON b.book_id = bk.id
         WHERE b.due_date < $1 AND b.returned_at IS NULL",
         now
    )
    .fetch_all(&pool)
    .await?;

    let overdue = rows.into_iter().map(|r| OverdueBorrowing {
        borrowing_id: r.borrowing_id,
        book_id: r.book_id,
        book_title: r.book_title,
        book_author: r.book_author,
        borrower_name: r.borrower_name,
        borrowed_at: r.borrowed_at,
        due_date: r.due_date,
    }).collect();

    Ok(Json(overdue))
}

#[cfg(test)]
mod tests;