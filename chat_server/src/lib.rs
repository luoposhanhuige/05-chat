// Is it appropriate to define the router in lib.rs?
// Yes, absolutely. In fact, this is considered a best practice in the Rust web development ecosystem (especially with Axum).

// Here is why it's a great approach:

// Separation of Concerns: Your main.rs should be as minimal as possible, usually just loading config, setting up tracing/logging, binding the TCP listener, and calling the router from your library.
// Testability: By having get_router in lib.rs, you can easily write unit/integration tests for your endpoints without having to start up an actual TCP server. You can pass mock configurations to get_router and use Axum's oneshot testing utilities to send simulated HTTP requests directly to the router.
// Reusability: If you ever need to embed this server into another application, or split it into different workspaces, having the core logic in a library crate makes it highly modular.
// As your application grows, you might extract the router into its own file (e.g., src/router.rs) and just export it through lib.rs, but the fundamental principle of keeping it out of main.rs is correct.

mod config;
mod handlers;

use handlers::*;
use std::{ops::Deref, sync::Arc};

use axum::{
    routing::{get, patch, post},
    Router,
};

pub use config::AppConfig; // Re-export AppConfig so that users of the library can access it directly via chat_server::AppConfig instead of chat_server::config::AppConfig.

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}

// This design pattern is commonly known in the Rust community as the "Inner Pattern" (or sometimes the "Shared State Pattern").

// It is a specialized form of the Wrapper/Facade Pattern designed specifically for Rust's ownership and concurrency model.

// Wrapping AppStateInner inside an Arc (Atomic Reference Counted pointer) serves two critical functions in an asynchronous web framework like Axum:

// Safe Sharing Across Threads: The underlying Tokio runtime routes concurrent HTTP requests across multiple OS threads. Arc allows multiple request handlers to hold a safe reference to the exact same memory simultaneously.
// Cheap Cloning: Axum requires the state to implement the Clone trait because it provides a distinct copy of the state to every single incoming request. By putting the heavy data (AppStateInner) inside an Arc, cloning AppState simply increments a fast atomic counter rather than deeply copying the configuration, database connection pools, or other shared resources.

// Yes. AppState wraps AppConfig (inside an Arc via AppStateInner) to safely and efficiently share the configuration across multiple asynchronous request handlers running on different OS threads.

// As your application grows, AppStateInner will likely be expanded to hold other shared resources that need concurrent access, such as database connection pools or Redis clients.

#[allow(unused)]
#[derive(Debug)]
pub(crate) struct AppStateInner {
    pub(crate) config: AppConfig,
}

// pub fn makes the function visible to outside consumers (like main.rs).
// get_router is the function that sets up the Axum router with all the defined routes and their corresponding handlers. It takes an AppConfig as an argument, which contains all the necessary configuration for the application (like server settings, database connections, etc.). The function then constructs the router and returns it ready to be served by the main function.
pub fn get_router(config: AppConfig) -> Router {
    let state = AppState::new(config);

    let api = Router::new()
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler))
        .route("/chat", get(list_chat_handler).post(create_chat_handler))
        .route(
            "/chat/{id}",
            patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/chat/{id}/messages", get(list_message_handler));

    Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state)
}

// 当我调用 state.config => state.inner.config
impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            inner: Arc::new(AppStateInner { config }),
        }
    }
}
