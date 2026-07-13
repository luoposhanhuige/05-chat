mod config;
mod error;
mod handlers;
mod models;
mod utils;

use anyhow::Context;
use handlers::*;
use sqlx::PgPool;
use std::{fmt, ops::Deref, sync::Arc};
use utils::{DecodingKey, EncodingKey};

// Re-export means you're making an item from a submodule available at a higher level so users don't need to know the internal structure. For example, by re-exporting AppError and User in lib.rs, other parts of the app can simply use crate::AppError and crate::User instead of having to import them from their specific submodules (e.g., crate::error::AppError or crate::models::User). This creates a cleaner and more intuitive public API for the library.
pub use error::{AppError, ErrorOutput};
pub use models::User;

use axum::{
    routing::{get, patch, post},
    Router,
};

pub use config::AppConfig;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}

#[allow(unused)]
pub(crate) struct AppStateInner {
    pub(crate) config: AppConfig,
    pub(crate) dk: DecodingKey,
    pub(crate) ek: EncodingKey,
    pub(crate) pool: PgPool,
}
/*
both DecodingKey and EncodingKey are the wrappers of the actural keys not the keys themselves, because they have the load function to load the keys from string, and they also implement the Debug trait, which means they can be printed in the logs. If they were just the keys themselves, they would not have these functionalities.
*/

pub async fn get_router(config: AppConfig) -> Result<Router, AppError> {
    let state = AppState::try_new(config).await?;

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

    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);
    // Nest a Router at some path. This allows you to break your application into smaller pieces and compose them together.

    Ok(app)
}

// The compiler knows which handlers are in which module through the **module hierarchy and file structure**.

// Here's how it works:

// **File structure:**
// ```
// src/
// ├── lib.rs
// └── handlers/
//     └── auth.rs
// ```

// **In `handlers/` (implicit module file):**
// When you have a `handlers/` directory, Rust looks for `handlers/mod.rs`. If it doesn't exist, it treats the directory as a module namespace.

// **In auth.rs:**
// Functions like `signin_handler` and `signup_handler` are defined with `pub(crate)` visibility.

// **In lib.rs:**
// ```rust
// mod handlers;  // declares handlers as a submodule

// use handlers::*;  // re-exports ALL public items from handlers
// ```

// This `use handlers::*;` imports everything from the `handlers` module, but the compiler still knows the origin:
// - `signin_handler` comes from `handlers::auth::signin_handler`
// - `signup_handler` comes from `handlers::auth::signup_handler`

// **The chain:**
// ```
// lib.rs → handlers (mod) → auth.rs (submodule of handlers)
//                        ↓
//                   signin_handler
//                   signup_handler
// ```

// When you write `.route("/signin", post(signin_handler))`, the compiler resolves `signin_handler` through the `use handlers::*;` import, but internally tracks it as `handlers::auth::signin_handler`.

// **TL;DR:** Module path + file structure = compiler knows the exact location of each handler.

// 当我调用 state.config => state.inner.config
impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    pub async fn try_new(config: AppConfig) -> Result<Self, AppError> {
        let dk = DecodingKey::load(&config.auth.pk).context("load pk failed")?;
        let ek = EncodingKey::load(&config.auth.sk).context("load sk failed")?;
        let pool = PgPool::connect(&config.server.db_url)
            .await
            .context("connect to db failed")?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
            }),
        })
    }
}

impl fmt::Debug for AppStateInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppStateInner")
            .field("config", &self.config)
            .finish()
    }
}
