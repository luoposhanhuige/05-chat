#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;
    use anyhow::Result;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn signup_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "Hunter42");
        let ret = signup_handler(State(state), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), StatusCode::CREATED);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }

    #[tokio::test]
    async fn signup_duplicate_user_should_409() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "Hunter42");
        signup_handler(State(state.clone()), Json(input.clone())).await?;
        let ret = signup_handler(State(state.clone()), Json(input.clone()))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::CONFLICT);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;

        assert_eq!(ret.error, "email already exists: tchen@acme.org");
        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let name = "Alice";
        let email = "alice@acme.org";
        let password = "Hunter42";
        let user = CreateUser::new(name, email, password);
        User::create(&user, &state.pool).await?;
        let input = SigninUser::new(email, password);
        let ret = signin_handler(State(state), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), StatusCode::OK);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");

        Ok(())
    }

    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let email = "alice@acme.org";
        let password = "Hunter42";
        let input = SigninUser::new(email, password);
        let ret = signin_handler(State(state), Json(input))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "Invalid email or password");

        Ok(())
    }
}



Yes. The original auth tests had **two real issues**.

## Real problem

### 1. `AppState::new_for_test()` was the wrong test setup path
The auth tests used:

````rust
let (_tdb, state) = AppState::new_for_test(config).await?;
````

That helper was the source of the unstable test database setup in your auth tests.  
The working pattern in this project is:

1. create `TestPg` directly from the base Postgres URL
2. replace `config.server.db_url` with `tdb.url()`
3. build `AppState` with `AppState::try_new(config)`

That is what the fixed `new_test_state()` does.

---

### 2. The temporary database was being dropped while the pool was still alive
In the original tests, `_tdb` was not explicitly cleaned up. It was dropped automatically at the end of the test.

But `state` contains a `PgPool`. If the temporary database `TestPg` is dropped while the pool is still open, `sqlx_db_tester` can panic in its `Drop` code. That is why you saw:

- `panic in a destructor during cleanup`
- `thread caused non-unwinding panic. aborting`
- `SIGABRT`

So the visible crash was **not** the real test failure. It was a cleanup crash.

---

## Why the original code was fragile

In the original code, patterns like this made cleanup unreliable:

````rust
let ret = signup_handler(State(state), Json(input))
    .await?
    .into_response();
````

Problems:

- `?` can return early before cleanup runs.
- `state` is moved into the handler call, so it cannot be closed afterward.
- `_tdb` is left to implicit drop at test end.

---

## The solution provided

The fixed auth.rs solves it in four ways.

### 1. Create the test database directly
Instead of `AppState::new_for_test()`:

````rust
async fn new_test_state() -> Result<(TestPg, AppState)> {
    let mut config = AppConfig::load()?;
    let tdb = TestPg::new(config.server.db_url.clone(), Path::new("../migrations"));
    config.server.db_url = tdb.url();
    let state = AppState::try_new(config).await?;
    Ok((tdb, state))
}
````

This uses the same stable setup pattern as the passing tests.

---

### 2. Keep `state` available for cleanup
The handler now receives `state.clone()` instead of consuming `state`:

````rust
let response = signup_handler(State(state.clone()), Json(input))
    .await
    .into_response();
````

So the original `state` is still available later.

---

### 3. Explicitly close the pool before dropping the temp database
Cleanup is now controlled:

````rust
async fn cleanup(state: AppState, tdb: TestPg) {
    state.pool.close().await;
    drop(state);
    drop(tdb);
}
````

This is the key fix for the destructor panic.

---

### 4. Delay `?` until after cleanup
The response body is collected first, but the `Result` is not unwrapped until after cleanup:

````rust
let body_result = response.into_body().collect().await.map(|c| c.to_bytes());

cleanup(state, tdb).await;

let body = body_result?;
````

So even if body parsing fails, cleanup still happens first.

---

## Short version

The original auth tests failed because:

- the test DB setup path via `AppState::new_for_test()` was problematic
- `TestPg` was dropped while `PgPool` was still alive
- some `?` paths could exit before cleanup

The fix was:

- stop using `AppState::new_for_test()` in auth.rs
- create `TestPg` directly
- override `config.server.db_url` with `tdb.url()`
- build state with `AppState::try_new()`
- explicitly call `state.pool.close().await` before dropping `tdb`

That is why the tests pass now.





