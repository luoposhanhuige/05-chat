use crate::{
    models::{CreateUser, SigninUser},
    AppError, AppState, ErrorOutput, User,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

// Response struct that returns a JWT token after successful auth.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutput {
    token: String,
}
// Why AuthOutput is called a "response struct"
// AuthOutput represents the data structure you're sending back to the client in the response body. It's not the HTTP response itself—it's the JSON payload that goes inside the response. So it's called a "response struct" because it defines what data the response contains.

// 2. Signup Handler
// Takes user creation data (CreateUser): name, email, password
// Creates a new user in the database via User::create()
// Signs a JWT token using state.ek.sign(user)
// Returns HTTP 201 (CREATED) with the token
pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::create(&input, &state.pool).await?;
    let token = state.ek.sign(user)?;
    let body = Json(AuthOutput { token });
    Ok((StatusCode::CREATED, body))
}

// 3. Signin Handler
// Takes email + password (SigninUser)
// Verifies credentials via User::verify() → returns Option<User>
// If user exists: Signs & returns a JWT token (HTTP 200)
// If user doesn't exist: Returns HTTP 403 FORBIDDEN with error message
pub(crate) async fn signin_handler(
    State(state): State<AppState>, // Extractor 1: AppState from router state
    Json(input): Json<SigninUser>, // Extractor 2: JSON body parsed as SigninUser
) -> Result<impl IntoResponse, AppError> {
    let user = User::verify(&input, &state.pool).await?;

    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            Ok((StatusCode::OK, Json(AuthOutput { token })).into_response())
        }
        None => {
            let body = Json(ErrorOutput::new("Invalid email or password"));
            Ok((StatusCode::FORBIDDEN, body).into_response())
        }
    }
}

// Why .into_response() is sometimes needed
// // The `.into_response()` method is used to convert a value into a type that implements the `IntoResponse` trait.
// // In Axum, handler functions need to return a type that implements `IntoResponse`. While tuples like `(StatusCode, Json<T>)` automatically implement `IntoResponse`, sometimes you might want to explicitly call `.into_response()` for clarity or when returning a more complex type.
// This explicitly calls .into_response() because the two branches return different types:

// Some branch: (StatusCode, Json)
// None branch: (StatusCode, Json)
// Both need to be the same type for the match expression, so .into_response() converts them both to a unified Response type.

// Whether the handler can have multiple extractors as parameters?
// Yes, Axum allows you to use multiple extractors in a single handler function. Each extractor corresponds to a different part of the HTTP request (e.g., path parameters, query parameters, headers, body, etc.). You can combine them as needed in your handler's parameter list.
// Looking at your handlers, you can see they already use **multiple extractors as parameters**. Let me break it down:

// **Current `signin_handler` uses 2 extractors:**

// ````rust
// pub(crate) async fn signin_handler(
//     State(state): State<AppState>,  // Extractor 1: AppState from router state
//     Json(input): Json<SigninUser>,  // Extractor 2: JSON body parsed as SigninUser
// ) -> Result<impl IntoResponse, AppError> {
//     // ...
// }
// ````

// **Yes, you can add more extractors!** Axum provides many built-in extractors:

// ````rust
// // Example with more extractors:
// pub(crate) async fn example_handler(
//     State(state): State<AppState>,           // Router state
//     Json(body): Json<SigninUser>,            // Request body (JSON)
//     Query(params): Query<QueryParams>,       // URL query parameters (?key=value)
//     Path(id): Path<u32>,                     // Path parameters (/{id})
//     headers: HeaderMap,                      // All HTTP headers
//     ConnectInfo(addr): ConnectInfo<SocketAddr>, // Client IP address
// ) -> Result<impl IntoResponse, AppError> {
//     // Access all the data
//     Ok(Json(serde_json::json!({
//         "state": todo!(),
//         "body": body,
//         "query": params,
//         "path_id": id,
//         "headers": headers,
//         "client_ip": addr,
//     })))
// }
// ````

// **Common Axum extractors:**
// - `State<T>` - Shared application state
// - `Json<T>` - Deserialize request body as JSON
// - `Query<T>` - URL query parameters
// - `Path<T>` - URL path parameters
// - `HeaderMap` - All HTTP headers
// - `ConnectInfo<SocketAddr>` - Client connection info
// - `Request` - Raw HTTP request
// - `Extension<T>` - Custom extensions
// - `Form<T>` - Form data

// **Order matters!** Extractors are processed left-to-right. Body extractors (like `Json`) should generally come last since they consume the request body.

// ...existing code...
#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;
    use anyhow::Result;
    use http_body_util::BodyExt;
    use sqlx_db_tester::TestPg;
    use std::path::Path;

    // Short version
    // The original auth tests failed because:

    // the test DB setup path via AppState::new_for_test() was problematic
    // TestPg was dropped while PgPool was still alive
    // some ? paths could exit before cleanup
    // The fix was:

    // stop using AppState::new_for_test() in auth.rs
    // create TestPg directly
    // override config.server.db_url with tdb.url()
    // build state with AppState::try_new()
    // explicitly call state.pool.close().await before dropping tdb
    // That is why the tests pass now.

    // Real problem
    // 1. AppState::new_for_test() was the wrong test setup path
    // The auth tests used:

    // let (_tdb, state) = AppState::new_for_test(config).await?;

    // That helper was the source of the unstable test database setup in your auth tests.

    // The working pattern in this project is:

    // create TestPg directly from the base Postgres URL
    // replace config.server.db_url with tdb.url()
    // build AppState with AppState::try_new(config)
    // That is what the fixed new_test_state() does.

    // 2. The temporary database was being dropped while the pool was still alive
    // In the original tests, _tdb was not explicitly cleaned up. It was dropped automatically at the end of the test.

    // But state contains a PgPool. If the temporary database TestPg is dropped while the pool is still open, sqlx_db_tester can panic in its Drop code. That is why you saw:

    // panic in a destructor during cleanup
    // thread caused non-unwinding panic. aborting
    // SIGABRT
    // So the visible crash was not the real test failure. It was a cleanup crash.

    // Creates isolated test database & app state
    async fn new_test_state() -> Result<(TestPg, AppState)> {
        let mut config = AppConfig::load()?;
        let tdb = TestPg::new(config.server.db_url.clone(), Path::new("../migrations"));
        config.server.db_url = tdb.url();
        let state = AppState::try_new(config).await?;
        Ok((tdb, state))
    }

    // Properly closes pool before dropping test DB (prevents crashes)
    async fn cleanup(state: AppState, tdb: TestPg) {
        state.pool.close().await;
        drop(state);
        drop(tdb);
    }

    // Creates user successfully, checks token is returned
    #[tokio::test]
    async fn signup_should_work() -> Result<()> {
        let (tdb, state) = new_test_state().await?;
        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "Hunter42");

        let response = signup_handler(State(state.clone()), Json(input))
            .await
            .into_response();
        let status = response.status();
        let body_result = response.into_body().collect().await.map(|c| c.to_bytes());

        cleanup(state, tdb).await;

        let body = body_result?;
        assert_eq!(status, StatusCode::CREATED);
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }

    // Attempts duplicate signup, expects CONFLICT (409)
    #[tokio::test]
    async fn signup_duplicate_user_should_409() -> Result<()> {
        let (tdb, state) = new_test_state().await?;
        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "Hunter42");

        let _ = signup_handler(State(state.clone()), Json(input.clone())).await;
        let response = signup_handler(State(state.clone()), Json(input))
            .await
            .into_response();

        let status = response.status();
        let body_result = response.into_body().collect().await.map(|c| c.to_bytes());

        cleanup(state, tdb).await;

        let body = body_result?;
        assert_eq!(status, StatusCode::CONFLICT);
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "email already exists: tchen@acme.org");
        Ok(())
    }

    // Logs in existing user, checks token is returned
    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let (tdb, state) = new_test_state().await?;
        let name = "Alice";
        let email = "alice@acme.org";
        let password = "Hunter42";
        let user = CreateUser::new(name, email, password);

        User::create(&user, &state.pool).await?;

        let input = SigninUser::new(email, password);
        let response = signin_handler(State(state.clone()), Json(input))
            .await
            .into_response();

        let status = response.status();
        let body_result = response.into_body().collect().await.map(|c| c.to_bytes());

        cleanup(state, tdb).await;

        let body = body_result?;
        assert_eq!(status, StatusCode::OK);
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }

    // Tries to login non-existent user, expects FORBIDDEN (403)
    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> Result<()> {
        let (tdb, state) = new_test_state().await?;
        let input = SigninUser::new("alice@acme.org", "Hunter42");

        let response = signin_handler(State(state.clone()), Json(input))
            .await
            .into_response();

        let status = response.status();
        let body_result = response.into_body().collect().await.map(|c| c.to_bytes());

        cleanup(state, tdb).await;

        let body = body_result?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "Invalid email or password");
        Ok(())
    }
}
// ...existing code...
