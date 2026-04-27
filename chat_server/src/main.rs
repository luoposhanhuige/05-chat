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
use chat_server::{get_router, AppConfig}; // When compiling main.rs, Cargo treats lib.rs as an external dependency automatically named after your package (in this case, chat_server). So you can import items from lib.rs using the crate name as a prefix (chat_server::AppConfig and chat_server::get_router).
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load()?;
    let addr = format!("0.0.0.0:{}", config.server.port);

    let app = get_router(config);
    let listener = TcpListener::bind(&addr).await?; // Rust asks the OS to create a socket. The OS allocates a port and binds it to the socket. The listener is now ready to accept incoming connections on that port.
    info!("Listening on: {}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
