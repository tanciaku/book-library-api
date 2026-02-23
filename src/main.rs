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
    year: Option<u32>,
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
) -> Json<PaginatedResponse<Book>> {
    let books = store.read().unwrap();

    let filtered: Vec<Book> = books
        .iter()
        .filter(|book| matches_filters(book, &params))
        .cloned()
        .collect();

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100);

    let total_items = filtered.len();
    let total_pages = (total_items + limit - 1) / limit;

    let start = (page - 1) * limit;
    let end = (start + limit).min(total_items);

    let paginated_data = if start < total_items {
        filtered[start..end].to_vec()
    } else {
        Vec::new()
    };

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
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
    let books = store.read().unwrap();

    let book = books.iter()
        .find(|t| t.id == id)
        .cloned();

    match book {
        Some(book) => Ok((StatusCode::OK, Json(book))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        ))),
    }
}

async fn update_book(
    State(store): State<BookStore>,
    Path(id): Path<u32>,
    Json(input): Json<UpdateBook>
) -> Result<(StatusCode, Json<Book>), (StatusCode, Json<ErrorResponse>)> {
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
            Ok((StatusCode::OK, Json(book.clone())))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        ))),
    }
}

async fn delete_book(
    State(store): State<BookStore>,
    Path(id): Path<u32>,
) -> Result<(StatusCode, ()), (StatusCode, Json<ErrorResponse>)> {
    let mut books = store.write().unwrap();

    let original_len = books.len();
    books.retain(|b| b.id != id);

    if books.len() < original_len {
        Ok((StatusCode::NO_CONTENT, ()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Book with ID {} not found", id) }
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use axum::http::{self, Request};

    fn fresh_app() -> Router {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));
        Router::new()
            .route("/health", get(health_check))
            .route("/books", get(list_books).post(add_book))
            .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
            .with_state(store)
    }

    fn app_with_books(books: Vec<Book>) -> Router {
        let store: BookStore = Arc::new(RwLock::new(books));
        Router::new()
            .route("/health", get(health_check))
            .route("/books", get(list_books).post(add_book))
            .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
            .with_state(store)
    }

    async fn send(app: Router, req: Request<Body>) -> (http::StatusCode, Vec<u8>) {
        let response = app.oneshot(req).await.unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, body)
    }

    fn sample_book(id: u32) -> Book {
        Book {
            id,
            title: format!("Book {}", id),
            author: "Author Name".to_string(),
            year: 2020,
            isbn: "9781593278281".to_string(),
            available: true,
        }
    }

    // --- health_check ---

    #[tokio::test]
    async fn health_check_returns_ok() {
        let app = fresh_app();
        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"OK");
    }

    // --- list_books ---

    #[tokio::test]
    async fn list_books_empty_store() {
        let app = fresh_app();
        let req = Request::builder().uri("/books").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert!(resp.data.is_empty());
        assert_eq!(resp.pagination.total_items, 0);
        assert_eq!(resp.pagination.total_pages, 0);
    }

    #[tokio::test]
    async fn list_books_returns_all() {
        let app = app_with_books(vec![sample_book(1), sample_book(2), sample_book(3)]);
        let req = Request::builder().uri("/books").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.data.len(), 3);
        assert_eq!(resp.pagination.total_items, 3);
    }

    #[tokio::test]
    async fn list_books_filter_by_author_case_insensitive() {
        let mut book1 = sample_book(1);
        book1.author = "Tolkien".to_string();
        let mut book2 = sample_book(2);
        book2.author = "Martin".to_string();
        let app = app_with_books(vec![book1, book2]);
        let req = Request::builder().uri("/books?author=tolkien").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].author, "Tolkien");
    }

    #[tokio::test]
    async fn list_books_filter_by_availability() {
        let mut book1 = sample_book(1);
        book1.available = true;
        let mut book2 = sample_book(2);
        book2.available = false;
        let app = app_with_books(vec![book1, book2]);
        let req = Request::builder().uri("/books?available=false").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert!(!resp.data[0].available);
    }

    #[tokio::test]
    async fn list_books_filter_by_year() {
        let mut book1 = sample_book(1);
        book1.year = 2010;
        let mut book2 = sample_book(2);
        book2.year = 2020;
        let app = app_with_books(vec![book1, book2]);
        let req = Request::builder().uri("/books?year=2010").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].year, 2010);
    }

    #[tokio::test]
    async fn list_books_pagination_second_page() {
        let books: Vec<Book> = (1..=15).map(sample_book).collect();
        let app = app_with_books(books);
        let req = Request::builder().uri("/books?page=2&limit=5").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.data.len(), 5);
        assert_eq!(resp.pagination.page, 2);
        assert_eq!(resp.pagination.limit, 5);
        assert_eq!(resp.pagination.total_items, 15);
        assert_eq!(resp.pagination.total_pages, 3);
        assert_eq!(resp.data[0].id, 6);
    }

    #[tokio::test]
    async fn list_books_page_beyond_total_returns_empty() {
        let books: Vec<Book> = (1..=3).map(sample_book).collect();
        let app = app_with_books(books);
        let req = Request::builder().uri("/books?page=99&limit=10").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert!(resp.data.is_empty());
    }

    #[tokio::test]
    async fn list_books_limit_capped_at_100() {
        let books: Vec<Book> = (1..=110).map(sample_book).collect();
        let app = app_with_books(books);
        let req = Request::builder().uri("/books?limit=200").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.pagination.limit, 100);
        assert_eq!(resp.data.len(), 100);
    }

    // --- add_book ---

    #[tokio::test]
    async fn add_book_returns_201_with_book() {
        let app = fresh_app();
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"The Rust Programming Language","author":"Steve Klabnik","year":2018,"isbn":"9781593278281"}"#))
            .unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::CREATED);
        let book: Book = serde_json::from_slice(&body).unwrap();
        assert_eq!(book.title, "The Rust Programming Language");
        assert_eq!(book.author, "Steve Klabnik");
        assert_eq!(book.year, 2018);
        assert_eq!(book.id, 1);
        assert!(book.available);
    }

    #[tokio::test]
    async fn add_book_isbn_with_dashes_accepted() {
        let app = fresh_app();
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Test","author":"Author","year":2020,"isbn":"978-1593278281"}"#))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn add_book_invalid_isbn_returns_400() {
        let app = fresh_app();
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Test","author":"Author","year":2020,"isbn":"bad-isbn"}"#))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_book_empty_title_returns_400() {
        let app = fresh_app();
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"","author":"Author","year":2020,"isbn":"9781593278281"}"#))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_book_empty_author_returns_400() {
        let app = fresh_app();
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Test","author":"","year":2020,"isbn":"9781593278281"}"#))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_book_future_year_returns_400() {
        let app = fresh_app();
        let future_year = chrono::Utc::now().year() + 1;
        let body = format!(
            r#"{{"title":"Future Book","author":"Someone","year":{},"isbn":"9781593278281"}}"#,
            future_year
        );
        let req = Request::builder()
            .method("POST")
            .uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // --- get_book ---

    #[tokio::test]
    async fn get_book_existing_returns_book() {
        let app = app_with_books(vec![sample_book(1)]);
        let req = Request::builder().uri("/books/1").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let book: Book = serde_json::from_slice(&body).unwrap();
        assert_eq!(book.id, 1);
    }

    #[tokio::test]
    async fn get_book_not_found_returns_404() {
        let app = fresh_app();
        let req = Request::builder().uri("/books/99").body(Body::empty()).unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert!(err.error.contains("99"));
    }

    // --- update_book ---

    #[tokio::test]
    async fn update_book_title() {
        let app = app_with_books(vec![sample_book(1)]);
        let req = Request::builder()
            .method("PUT")
            .uri("/books/1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Updated Title"}"#))
            .unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let book: Book = serde_json::from_slice(&body).unwrap();
        assert_eq!(book.title, "Updated Title");
    }

    #[tokio::test]
    async fn update_book_partial_update_preserves_other_fields() {
        let app = app_with_books(vec![sample_book(1)]);
        let req = Request::builder()
            .method("PUT")
            .uri("/books/1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"available":false}"#))
            .unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::OK);
        let book: Book = serde_json::from_slice(&body).unwrap();
        assert!(!book.available);
        assert_eq!(book.title, "Book 1");
        assert_eq!(book.author, "Author Name");
        assert_eq!(book.year, 2020);
    }

    #[tokio::test]
    async fn update_book_not_found_returns_404() {
        let app = fresh_app();
        let req = Request::builder()
            .method("PUT")
            .uri("/books/99")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Whatever"}"#))
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // --- delete_book ---

    #[tokio::test]
    async fn delete_book_existing_returns_204() {
        let app = app_with_books(vec![sample_book(1)]);
        let req = Request::builder()
            .method("DELETE")
            .uri("/books/1")
            .body(Body::empty())
            .unwrap();
        let (status, _) = send(app, req).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_book_not_found_returns_404() {
        let app = fresh_app();
        let req = Request::builder()
            .method("DELETE")
            .uri("/books/99")
            .body(Body::empty())
            .unwrap();
        let (status, body) = send(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert!(err.error.contains("99"));
    }

    // --- integration ---

    fn shared_app(store: BookStore) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/books", get(list_books).post(add_book))
            .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
            .with_state(store)
    }

    #[tokio::test]
    async fn integration_create_then_get() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let post_req = Request::builder()
            .method("POST").uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Dune","author":"Frank Herbert","year":1965,"isbn":"9780340960196"}"#))
            .unwrap();
        let (post_status, post_body) = send(shared_app(store.clone()), post_req).await;
        assert_eq!(post_status, StatusCode::CREATED);
        let created: Book = serde_json::from_slice(&post_body).unwrap();

        let get_req = Request::builder()
            .method("GET").uri(format!("/books/{}", created.id))
            .body(Body::empty()).unwrap();
        let (get_status, get_body) = send(shared_app(store.clone()), get_req).await;
        assert_eq!(get_status, StatusCode::OK);
        let fetched: Book = serde_json::from_slice(&get_body).unwrap();

        assert_eq!(created.id,     fetched.id);
        assert_eq!(created.title,  fetched.title);
        assert_eq!(created.author, fetched.author);
        assert_eq!(created.year,   fetched.year);
        assert_eq!(created.isbn,   fetched.isbn);
    }

    #[tokio::test]
    async fn integration_create_update_get() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let post_req = Request::builder()
            .method("POST").uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Original Title","author":"Jane Doe","year":2000,"isbn":"9780340960196"}"#))
            .unwrap();
        let (_, post_body) = send(shared_app(store.clone()), post_req).await;
        let created: Book = serde_json::from_slice(&post_body).unwrap();

        let put_req = Request::builder()
            .method("PUT").uri(format!("/books/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Updated Title","available":false}"#))
            .unwrap();
        let (put_status, _) = send(shared_app(store.clone()), put_req).await;
        assert_eq!(put_status, StatusCode::OK);

        let get_req = Request::builder()
            .method("GET").uri(format!("/books/{}", created.id))
            .body(Body::empty()).unwrap();
        let (_, get_body) = send(shared_app(store.clone()), get_req).await;
        let final_book: Book = serde_json::from_slice(&get_body).unwrap();

        assert_eq!(final_book.title,     "Updated Title");
        assert_eq!(final_book.available, false);
        assert_eq!(final_book.author,    "Jane Doe");
        assert_eq!(final_book.year,      2000);
    }

    #[tokio::test]
    async fn integration_create_delete_then_get_returns_404() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let post_req = Request::builder()
            .method("POST").uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Temporary","author":"Someone","year":2021,"isbn":"9780340960196"}"#))
            .unwrap();
        let (_, post_body) = send(shared_app(store.clone()), post_req).await;
        let created: Book = serde_json::from_slice(&post_body).unwrap();

        let del_req = Request::builder()
            .method("DELETE").uri(format!("/books/{}", created.id))
            .body(Body::empty()).unwrap();
        let (del_status, _) = send(shared_app(store.clone()), del_req).await;
        assert_eq!(del_status, StatusCode::NO_CONTENT);

        let get_req = Request::builder()
            .method("GET").uri(format!("/books/{}", created.id))
            .body(Body::empty()).unwrap();
        let (get_status, _) = send(shared_app(store.clone()), get_req).await;
        assert_eq!(get_status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn integration_multiple_creates_reflected_in_list() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let payloads = [
            r#"{"title":"Book A","author":"Author A","year":2001,"isbn":"9780340960196"}"#,
            r#"{"title":"Book B","author":"Author B","year":2002,"isbn":"9780340960196"}"#,
            r#"{"title":"Book C","author":"Author C","year":2003,"isbn":"9780340960196"}"#,
        ];

        for payload in &payloads {
            let req = Request::builder()
                .method("POST").uri("/books")
                .header("content-type", "application/json")
                .body(Body::from(*payload)).unwrap();
            let (status, _) = send(shared_app(store.clone()), req).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        let list_req = Request::builder()
            .method("GET").uri("/books")
            .body(Body::empty()).unwrap();
        let (list_status, list_body) = send(shared_app(store.clone()), list_req).await;
        assert_eq!(list_status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&list_body).unwrap();

        assert_eq!(resp.pagination.total_items, 3);
        assert_eq!(resp.data.len(), 3);

        let titles: Vec<&str> = resp.data.iter().map(|b| b.title.as_str()).collect();
        assert!(titles.contains(&"Book A"));
        assert!(titles.contains(&"Book B"));
        assert!(titles.contains(&"Book C"));
    }

    #[tokio::test]
    async fn integration_update_availability_then_filter() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let post_req = Request::builder()
            .method("POST").uri("/books")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Loanable","author":"Lib Author","year":2015,"isbn":"9780340960196"}"#))
            .unwrap();
        let (_, post_body) = send(shared_app(store.clone()), post_req).await;
        let created: Book = serde_json::from_slice(&post_body).unwrap();
        assert!(created.available);

        let put_req = Request::builder()
            .method("PUT").uri(format!("/books/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"available":false}"#)).unwrap();
        let (put_status, _) = send(shared_app(store.clone()), put_req).await;
        assert_eq!(put_status, StatusCode::OK);

        let avail_req = Request::builder()
            .method("GET").uri("/books?available=true")
            .body(Body::empty()).unwrap();
        let (_, avail_body) = send(shared_app(store.clone()), avail_req).await;
        let avail_resp: PaginatedResponse<Book> = serde_json::from_slice(&avail_body).unwrap();
        assert!(avail_resp.data.is_empty());

        let unavail_req = Request::builder()
            .method("GET").uri("/books?available=false")
            .body(Body::empty()).unwrap();
        let (_, unavail_body) = send(shared_app(store.clone()), unavail_req).await;
        let unavail_resp: PaginatedResponse<Book> = serde_json::from_slice(&unavail_body).unwrap();
        assert_eq!(unavail_resp.data.len(), 1);
        assert_eq!(unavail_resp.data[0].id, created.id);
    }

    #[tokio::test]
    async fn integration_create_multiple_then_filter_by_author() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let payloads = [
            r#"{"title":"T1","author":"George Orwell","year":1949,"isbn":"9780340960196"}"#,
            r#"{"title":"T2","author":"George R.R. Martin","year":1996,"isbn":"9780340960196"}"#,
            r#"{"title":"T3","author":"Isaac Asimov","year":1951,"isbn":"9780340960196"}"#,
        ];

        for payload in &payloads {
            let req = Request::builder()
                .method("POST").uri("/books")
                .header("content-type", "application/json")
                .body(Body::from(*payload)).unwrap();
            send(shared_app(store.clone()), req).await;
        }

        let filter_req = Request::builder()
            .method("GET").uri("/books?author=george")
            .body(Body::empty()).unwrap();
        let (status, body) = send(shared_app(store.clone()), filter_req).await;
        assert_eq!(status, StatusCode::OK);
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.data.len(), 2);
        for book in &resp.data {
            assert!(book.author.to_lowercase().contains("george"));
        }
    }

    #[tokio::test]
    async fn integration_create_many_then_paginate() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        // Create 12 books via the API
        for i in 1..=12u32 {
            let payload = format!(
                r#"{{"title":"Paginated Book {}","author":"Paged Author","year":2020,"isbn":"9780340960196"}}"#,
                i
            );
            let req = Request::builder()
                .method("POST").uri("/books")
                .header("content-type", "application/json")
                .body(Body::from(payload)).unwrap();
            let (status, _) = send(shared_app(store.clone()), req).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        // Page 1: expect 5 books
        let req = Request::builder()
            .method("GET").uri("/books?page=1&limit=5")
            .body(Body::empty()).unwrap();
        let (status, body) = send(shared_app(store.clone()), req).await;
        assert_eq!(status, StatusCode::OK);
        let page1: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(page1.data.len(), 5);
        assert_eq!(page1.pagination.page, 1);
        assert_eq!(page1.pagination.limit, 5);
        assert_eq!(page1.pagination.total_items, 12);
        assert_eq!(page1.pagination.total_pages, 3);
        assert_eq!(page1.data[0].title, "Paginated Book 1");
        assert_eq!(page1.data[4].title, "Paginated Book 5");

        // Page 2: expect 5 books
        let req = Request::builder()
            .method("GET").uri("/books?page=2&limit=5")
            .body(Body::empty()).unwrap();
        let (status, body) = send(shared_app(store.clone()), req).await;
        assert_eq!(status, StatusCode::OK);
        let page2: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(page2.data.len(), 5);
        assert_eq!(page2.pagination.page, 2);
        assert_eq!(page2.data[0].title, "Paginated Book 6");
        assert_eq!(page2.data[4].title, "Paginated Book 10");

        // Page 3: expect 2 remaining books
        let req = Request::builder()
            .method("GET").uri("/books?page=3&limit=5")
            .body(Body::empty()).unwrap();
        let (status, body) = send(shared_app(store.clone()), req).await;
        assert_eq!(status, StatusCode::OK);
        let page3: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert_eq!(page3.data.len(), 2);
        assert_eq!(page3.pagination.page, 3);
        assert_eq!(page3.data[0].title, "Paginated Book 11");
        assert_eq!(page3.data[1].title, "Paginated Book 12");

        // Page 4: beyond total — expect empty data
        let req = Request::builder()
            .method("GET").uri("/books?page=4&limit=5")
            .body(Body::empty()).unwrap();
        let (status, body) = send(shared_app(store.clone()), req).await;
        assert_eq!(status, StatusCode::OK);
        let page4: PaginatedResponse<Book> = serde_json::from_slice(&body).unwrap();
        assert!(page4.data.is_empty());
        assert_eq!(page4.pagination.total_items, 12);
        assert_eq!(page4.pagination.total_pages, 3);

        // Confirm no overlap between pages 1 and 2
        let ids_page1: Vec<u32> = page1.data.iter().map(|b| b.id).collect();
        let ids_page2: Vec<u32> = page2.data.iter().map(|b| b.id).collect();
        assert!(ids_page1.iter().all(|id| !ids_page2.contains(id)));
    }

    #[tokio::test]
    async fn integration_delete_one_of_many_leaves_rest_intact() {
        let store: BookStore = Arc::new(RwLock::new(Vec::new()));

        let mut ids = Vec::new();
        for i in 0..3u32 {
            let payload = format!(
                r#"{{"title":"Book {}","author":"Author","year":2020,"isbn":"9780340960196"}}"#, i
            );
            let req = Request::builder()
                .method("POST").uri("/books")
                .header("content-type", "application/json")
                .body(Body::from(payload)).unwrap();
            let (_, body) = send(shared_app(store.clone()), req).await;
            let book: Book = serde_json::from_slice(&body).unwrap();
            ids.push(book.id);
        }

        let del_req = Request::builder()
            .method("DELETE").uri(format!("/books/{}", ids[1]))
            .body(Body::empty()).unwrap();
        let (del_status, _) = send(shared_app(store.clone()), del_req).await;
        assert_eq!(del_status, StatusCode::NO_CONTENT);

        let list_req = Request::builder()
            .method("GET").uri("/books")
            .body(Body::empty()).unwrap();
        let (_, list_body) = send(shared_app(store.clone()), list_req).await;
        let resp: PaginatedResponse<Book> = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(resp.pagination.total_items, 2);

        let remaining_ids: Vec<u32> = resp.data.iter().map(|b| b.id).collect();
        assert!(remaining_ids.contains(&ids[0]));
        assert!(remaining_ids.contains(&ids[2]));
        assert!(!remaining_ids.contains(&ids[1]));
    }
}