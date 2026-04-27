mod sse;

use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};
use sse::sse_handler;

const INDEX_HTML: &str = include_str!("../index.html");

pub fn get_router() -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/events", get(sse_handler))
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

// include_str!("../index.html");
// If you used const INDEX_HTML: &str = "../index.html";, the value of INDEX_HTML would literally just be the 13-character string "../index.html". When a user visits your site, their browser would just display the text "../index.html" instead of rendering the webpage.

// By using the include_str! macro:

// Compile-time reading: The Rust compiler reads the actual contents of the index.html file during compilation.
// Binary Embedding: It embeds the entire HTML source code directly into your compiled executable binary.
// No runtime dependencies: When you run the server, it doesn't need to look for index.html on the hard drive. The HTML is already in memory and ready to be served by the index_handler.

// `Html(INDEX_HTML)` wraps the raw HTML string inside Axum's `Html` response type.

// By default, if you return a plain string from an Axum handler, the framework sets the HTTP header `Content-Type: text/plain`, causing the browser to display raw HTML tags instead of rendering them.

// Wrapping it in `Html(...)` implements Axum's `IntoResponse` trait specifically for HTML. It automatically sets the HTTP header to `Content-Type: text/html; charset=utf-8`, instructing the browser to correctly render the UI.
