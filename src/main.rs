use axum::{Json, Router, extract::{Path, Query, State}, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use chrono::Datelike;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
    id: u32,
    title: String,
    author: String,
    year: u32,
    isbn: String,
    available: bool,
}

#[derive(Debug, Deserialize)]
struct AddBook {
    title: String,
    author: String,
    year: u32,
    isbn: String,
}

#[derive(Debug, Deserialize)]
struct UpdateBook {
    title: Option<String>,
    author: Option<String>,
    year: Option<u32>,
    isbn: Option<String>,
    available: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BookParams {
    available: Option<bool>,
    author: Option<String>,
    year: Option<u32>
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

type BookStore = Arc<RwLock<Vec<Book>>>;

#[tokio::main]
async fn main() {
    let store: BookStore = Arc::new(RwLock::new(Vec::new()));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/books", get(list_books).post(add_book))
        .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("\n Server running on http://localhost:3000");
    println!("\n Available endpoints:");
    println!("  GET    /books       - List all books");
    println!("  POST   /books       - Add a book");
    println!("  GET    /books/:id   - Get a book");
    println!("  PUT    /books/:id   - Update a book");
    println!("  DELETE /books/:id   - Delete a book");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_books(
    State(store): State<BookStore>,
    Query(params): Query<BookParams>
) -> Json<Vec<Book>> {
    let books = store.read().unwrap();

    let filtered: Vec<Book> = books
        .iter()
        .filter(|book| matches_filters(book, &params))
        .cloned()
        .collect();

    Json(filtered)
}

fn matches_filters(book: &Book, params: &BookParams) -> bool {
    let availability_matches = params
        .available
        .map_or(true, |availability| book.available == availability);

    let author_matches = params
        .author
        .as_ref()
        .map_or(true, |search| author_matches_search(book.author.as_str(), search.as_str()));

    let year_matches = params
        .year
        .map_or(true, |year| book.year == year);

    availability_matches && author_matches && year_matches
}

fn author_matches_search(author: &str, search_term: &str) -> bool {
    author.to_lowercase().contains(&search_term.to_lowercase())
}

async fn add_book(
    State(store): State<BookStore>,
    Json(input): Json<AddBook>
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
    let mut books = store.write().unwrap();

    let new_id = books.len() as u32 + 1;

    if !validate_book(&input) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid book data. Check title, author, year, and ISBN format.".to_string()
            })
        ));
    }

    let book = Book {
        id: new_id,
        title: input.title,
        author: input.author,
        year: input.year,
        isbn: input.isbn,
        available: true,
    };

    books.push(book.clone());

    Ok((StatusCode::CREATED, Json(book)))
}

fn validate_book(book: &AddBook) -> bool {
    !book.title.is_empty() &&
    !book.author.is_empty() &&
    is_valid_year(book.year) &&
    is_valid_isbn(&book.isbn)
}

fn is_valid_year(year: u32) -> bool {
    let current_year = chrono::Utc::now().year(); 
    (1000..=current_year).contains(&(year as i32))
}

fn is_valid_isbn(isbn: &str) -> bool {
    let cleaned = isbn.replace("-", "");
    cleaned.len() == 13 && cleaned.chars().all(|c| c.is_numeric())
}

async fn get_book(
    State(store): State<BookStore>,
    Path(id): Path<u32>
) -> Result<Json<Book>, StatusCode> {
    let books = store.read().unwrap();

    let book = books.iter()
        .find(|t| t.id == id)
        .cloned();

    match book {
        Some(book) => Ok(Json(book)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_book(
    State(store): State<BookStore>,
    Path(id): Path<u32>,
    Json(input): Json<UpdateBook>
) -> Result<Json<Book>, StatusCode> {
    let mut books = store.write().unwrap();

    let book = books.iter_mut()
        .find(|b| b.id == id);

    match book {
        Some(book) => {
            input.title.map(|b| book.title = b);
            input.author.map(|b| book.author = b);
            input.year.map(|b| book.year = b);
            input.isbn.map(|b| book.isbn = b);
            input.available.map(|b| book.available = b);
            Ok(Json(book.clone()))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_book(
    State(store): State<BookStore>,
    Path(id): Path<u32>,
) -> StatusCode {
    let mut books = store.write().unwrap();

    let original_len = books.len();
    books.retain(|b| b.id != id);

    if books.len() < original_len {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}