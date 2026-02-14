use axum::{Json, Router, extract::{Path, State}, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
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

type BookStore = Arc<RwLock<Vec<Book>>>;

#[tokio::main]
async fn main() {
    let store: BookStore = Arc::new(RwLock::new(Vec::new()));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/books", get(list_books).post(add_book))
        .route("/books/{id}", get(get_book))
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_books(
    State(store): State<BookStore>
) -> Json<Vec<Book>> {
    let books = store.read().unwrap();
    Json(books.clone())
}

async fn add_book(
    State(store): State<BookStore>,
    Json(input): Json<AddBook>
) -> (StatusCode, Json<Book>) {
    let mut books = store.write().unwrap();

    let new_id = books.len() as u32 + 1;

    let book = Book {
        id: new_id,
        title: input.title,
        author: input.author,
        year: input.year,
        isbn: input.isbn,
        available: true,
    };

    books.push(book.clone());

    (StatusCode::CREATED, Json(book))
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