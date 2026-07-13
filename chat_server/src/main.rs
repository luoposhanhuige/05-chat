// cd chat/chat_server
// cargo run --bin chat_server
// curl http://localhost:6688/

// curl -X POST http://localhost:6688/api/signup \
//      -H "Content-Type: application/json" \
//      -d '{"username": "test_user", "password": "password123"}'

// curl http://localhost:6688/api/chat

// curl -X POST http://localhost:6688/api/chat \
//      -H "Content-Type: application/json" \
//      -d '{"name": "General Chat"}'

// curl -X POST http://localhost:6688/api/chat/1 \
//      -H "Content-Type: application/json" \
//      -d '{"text": "Hello world!"}'

// In Rust, a single package can contain both a binary crate (main.rs) and a library crate (src/lib.rs).

use anyhow::Result;
use chat_server::{get_router, AppConfig};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
// fmt::Layer gives you the concrete object to creat, layer::Layer as _ gives you the trait methods you can call on it

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load()?;
    let addr = format!("0.0.0.0:{}", config.server.port);

    let app = get_router(config).await?; // app is a Router instance that contains all the routes and handlers for the chat server. It is created by calling get_router with the loaded configuration.
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    axum::serve(listener, app.into_make_service()).await?;
    // If you're on Axum 0.7+, you can often skip .into_make_service() and just do:
    // axum::serve(listener, app).await?;

    // cargo tree | grep axum
    // cargo pkgid axum
    Ok(())
}

// Q:
// the axum::serve bridges the listener (the channel to/from client Request) and app.into_make_service (the routers which handle the request and make the result into Response)), right?

// A:
// Exactly right! You've got the concept down.

// **`axum::serve` bridges:**
// - **Left side**: `listener` - TCP connection channel that receives raw client requests
// - **Right side**: `app.into_make_service()` - Router that processes requests and generates responses

// **The flow:**
// 1. `TcpListener` waits for incoming TCP connections from clients
// 2. `axum::serve` accepts each connection and extracts the HTTP request
// 3. Passes it to the Router (`app`)
// 4. Router matches the request path/method to a handler
// 5. Handler processes it and returns a Response
// 6. `axum::serve` sends the Response back through the TCP connection to the client

// **Why `into_make_service()`?**
// - `app` is a `Router` (stateless route definitions)
// - `into_make_service()` converts it into a service factory that can create a new service instance for each connection
// - This allows concurrent request handling

// Think of it like: **Listener = post office receiving mail**, **Router = mail sorters**, **axum::serve = postal worker connecting them together**.
