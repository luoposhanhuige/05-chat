use axum::response::{Sse, sse::Event};
use axum_extra::{TypedHeader, headers};
use futures::{Stream, stream};
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt;
use tracing::info;

pub(crate) async fn sse_handler(
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("`{}` connected", user_agent.as_str());

    // A `Stream` that repeats an event every second
    //
    // You can also create streams from tokio channels using the wrappers in
    // https://docs.rs/tokio-stream
    let stream = stream::repeat_with(|| Event::default().data("hi!"))
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

// Sse<impl Stream<Item = Result<Event, Infallible>>>

// Here is the breakdown of the return type `Sse<impl Stream<Item = Result<Event, Infallible>>>`:

// ### 1. `Sse<...>` (The Wrapper)
// `Sse` is a struct provided by Axum (`axum::response::Sse`). Its job is to take a stream of data and automatically convert it into a compliant Server-Sent Events HTTP response. It sets the required `Content-Type: text/event-stream` header and manages the underlying connection.

// ### 2. `impl Stream<...>` (The Asynchronous Iterator)
// *   **`Stream`**: This is the asynchronous version of Rust's `Iterator`. Instead of yielding items immediately in a loop, a `Stream` yields items over time (e.g., waiting 1 second between each item).
// *   **`impl`**: Because chaining stream adapters (like `.map().throttle()`) creates an extremely complex, unreadable underlying type, `impl Stream` tells the compiler: *"I don't want to write the exact type name; just trust me that whatever I return implements the `Stream` trait."*

// ### 3. `Item = Result<Event, Infallible>` (The Yielded Data)
// A `Stream` dictates what type of data it yields via the `Item` associated type. Axum's `Sse` requires the stream to yield a `Result` so it knows how to handle mid-stream errors.

// *   **`Event`**: This is an Axum struct representing a single SSE message (e.g., `data: hi!\n\n`).
// *   **`Infallible`**: This is a special built-in Rust type (`std::convert::Infallible`) that represents an error that can *never* happen. By defining the error type as `Infallible`, you are guaranteeing to the compiler (and to Axum) that generating these `Event`s will never fail.

// **Summary:**
// Overall, this signature means: *"I am returning an Axum SSE response, backed by a stream of data yielded over time. Every item in this stream will successfully be an SSE `Event`, and it is mathematically impossible for the stream to produce an error."*

// In Rust, `as_str()` works because the developers of the `headers::UserAgent` struct explicitly implemented a method with that name for the type.

// Under the hood:
// 1. **Internal Storage:** The `UserAgent` struct internally stores the raw HTTP header value, typically as a `HeaderValue` (a type from the `http` crate that holds raw byte data).
// 2. **Borrowing:** When you call `.as_str()`, the method borrows this internal data, checks that it is valid text, and returns a reference to a string slice (`&str`).
// 3. **Zero-Cost:** Because it returns a reference (`&str`) rather than an owned `String`, this conversion involves no memory allocation or data copying. It simply points to the memory where the HTTP header was originally parsed.

// This code defines the asynchronous handler for the `/events` endpoint, responsible for maintaining the Server-Sent Events (SSE) connection and streaming data to the client.

// Here is the step-by-step breakdown:

// ### 1. The Function Signature & Extractor
// ```rust
// pub(crate) async fn sse_handler(
//     TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
// ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
// ```
// * **Extractor:** `TypedHeader(user_agent)` automatically extracts the `User-Agent` HTTP header from the incoming request so you can log what browser/client connected.
// * **Return Type:** It returns an `Sse` wrapper around a Rust `Stream`. A `Stream` is just the asynchronous version of an `Iterator`. It yields items `Result<Event, Infallible>` over time. (`Infallible` means this specific stream cannot produce an error).

// ### 2. Creating the Asynchronous Stream
// ```rust
// let stream = stream::repeat_with(|| Event::default().data("hi!"))
//     .map(Ok)
//     .throttle(Duration::from_secs(1));
// ```
// * **`stream::repeat_with(...)`**: Creates an infinite stream that constantly generates new SSE `Event` objects containing the text payload `"hi!"`.
// * **`.map(Ok)`**: Wraps each generated event in a `Result::Ok(Event)`.
// * **`.throttle(...)`**: This belongs to `tokio-stream`. It slows down the infinite stream, ensuring it only yields one `"hi!"` event exactly every 1 second. Without this, it would flood the client with millions of events per second.

// ### 3. Returning the SSE Response
// ```rust
// Sse::new(stream).keep_alive(
//     axum::response::sse::KeepAlive::new()
//         .interval(Duration::from_secs(1))
//         .text("keep-alive-text"),
// )
// ```
// * **`Sse::new(stream)`**: Wraps the stream into an HTTP response with the correct `Content-Type: text/event-stream` header.
// * **`.keep_alive(...)`**: Configures Axum to inject a hidden ping message (`:keep-alive-text`) every second. This is crucial for SSE because if no data is sent for a while, reverse proxies (like Nginx) or firewalls might assume the connection is dead and forcefully close it.

// ### 1. What is `UserAgent` and `headers::UserAgent`?
// * **`User-Agent` (HTTP Concept):** It is a standard HTTP header sent by browsers and clients. It contains a string identifying the browser, version, and operating system (e.g., `Mozilla/5.0 (Macintosh; Intel Mac OS X...)`).
// * **`headers::UserAgent` (Rust Type):** This is a strongly-typed Rust struct from the `headers` crate (re-exported by `axum_extra`). Instead of dealing with raw strings, this struct represents the parsed `User-Agent` header, ensuring type safety.

// ### 2. What does `TypedHeader<headers::UserAgent>` do?
// In Axum, `TypedHeader<T>` is an **Extractor**.
// When placed in a handler's arguments, Axum automatically steps in before your function runs and performs the following:
// 1. It looks at the incoming HTTP request.
// 2. It searches for the `User-Agent` header.
// 3. It parses the raw string into the `headers::UserAgent` struct.
// 4. If successful, it passes the typed struct into your function. (If the header is missing, Axum automatically rejects the request with a `400 Bad Request` error. You can wrap it in `Option<TypedHeader<...>>` to make it optional).

// ### 3. How many other items exist in `headers::`?
// The `headers` crate provides strongly-typed structs for dozens of standard HTTP headers. Some of the most common ones include:

// * **Authentication:** `Authorization`, `Cookie`
// * **Content Negotiation:** `Accept`, `AcceptLanguage`, `AcceptEncoding`
// * **Body Metadata:** `ContentType`, `ContentLength`
// * **Caching:** `CacheControl`, `ETag`, `LastModified`
// * **Routing/Context:** `Host`, `Referer`, `Origin`

// You can use any of these with `TypedHeader` to safely extract exactly what you need from the HTTP request.

// TypedHeader(user_agent): TypedHeader<headers::UserAgent>

// The `TypedHeader` extractor is a powerful feature of Axum that allows you to extract and parse specific HTTP headers into strongly-typed Rust structs.

// Axum parses the **type (`TypedHeader<headers::UserAgent>`) first**.

// Here is exactly how the process works in order:

// ### 1. The Type dictates the Extraction (Right side)
// Axum's extraction system is fully type-driven. Before your function even starts running, Axum looks at the type signature: `TypedHeader<headers::UserAgent>`.
// This type tells Axum's internal engine: *"Look at the incoming HTTP request, find the User-Agent header, parse it, and create an instance of the `TypedHeader` struct."*

// ### 2. Rust handles the Destructuring (Left side)
// Once Axum has successfully created that `TypedHeader` instance, standard Rust function arguments take over.
// The left side (`TypedHeader(user_agent)`) is just Rust **pattern matching**. It unwraps the tuple struct, takes the inner `UserAgent` value, and binds it to the local variable named `user_agent` so you can use it inside your function.

// **To summarize:**
// The right side tells Axum **how to get the data from the HTTP request**.
// The left side tells Rust **how to unpack that data into a variable**.

// Q: since the sse_handler is registered as the handler in the routers, the axum then seek its parameters for mapping to the corresponding extractors, right?

// Yes, that's correct! When you register `sse_handler` as the handler for the `/events` route in your Axum router, Axum will automatically inspect the parameters of the `sse_handler` function to determine which extractors to use.

// Yes, you are exactly right!

// When you register `sse_handler` in the Axum router (e.g., `get(sse_handler)`), Axum uses Rust's type system to inspect the function's parameters at compile time.

// Here is exactly what happens:

// 1. **Trait Resolution (Compile Time):** Axum checks if every parameter in your function implements the `FromRequest` or `FromRequestParts` trait. `TypedHeader` implements this trait, which contains the logic for reading HTTP headers.
// 2. **Execution (Runtime):** When an HTTP request hits that specific route, Axum intercepts it *before* it reaches your function.
// 3. **Extraction:** Axum executes the trait's extraction logic for each parameter one by one. It looks at the HTTP request, pulls out the `User-Agent` header, and parses it.
// 4. **Function Call:** Only if all extractors succeed does Axum actually call your `sse_handler`, passing in the fully parsed data as arguments.

// If an extractor fails (for example, if a required header is missing), Axum automatically cancels the function call and returns a `400 Bad Request` to the client.
