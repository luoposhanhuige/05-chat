// Grant the postgres user the correct privileges in your local database.
// psql -U xhui -d postgres -c "ALTER USER postgres CREATEDB;"
//          btw, I ran a command to check the existing users by explicitly connecting to the default postgres database (psql -U xhui -d postgres -c "\du").
// cargo test

use crate::{AppError, User};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::PgPool;
use std::mem;

// Summary
// create: Used exclusively in your POST /register route.
// verify: Used exclusively in your POST /login route.
// find_by_email: Used as a utility tool across many routes (validating the sign-up form dynamically, password resets, or checking user permissions before performing an action).

impl User {
    /// Find a user by email
    // Executes a SELECT query to fetch a user by their email address. It returns Ok(Option<User>), successfully handling cases where the user does not exist.
    // fetch_optional(pool): Expects zero or one row. If a row is found, it returns Ok(Some(User)). If no row is found, it returns Ok(None). It only returns an error if something actually goes wrong (like a syntax error or a broken connection). This is appropriate here because it's perfectly valid for a user to not exist with the given email, and we want to handle that case gracefully without treating it as an error.
    // find_by_email and verify: These return Result<Option<Self>, AppError> because it's completely normal for a user not to exist (e.g., searching for an unregistered email or entering a bad password). Finding nothing is not a system error; it's a valid empty result (None).
    pub async fn find_by_email(email: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user =
            sqlx::query_as("SELECT id, fullname, email, created_at FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(pool)
                .await?;
        Ok(user)
    }

    /// Create a new user
    // Hashes the provided plain-text password, then executes an INSERT statement to add a new row to the database. It returns the newly created User.
    // fetch_one(pool): Expects exactly one row. If a row is found, it returns Ok(User). If no row is found, it returns an Error. This is appropriate here because the RETURNING clause guarantees that a row will be returned if the insert is successful. If something goes wrong (like a constraint violation), it will return an error.
    // create: This returns Result<Self, AppError> because an INSERT statement is expected to create exactly one record. If it succeeds, you definitely get a User back. If it fails (e.g., the email already exists, triggering a database constraint), it returns a system error (AppError).
    pub async fn create(
        email: &str,
        fullname: &str,
        password: &str,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        let password_hash = hash_password(password)?;
        let user = sqlx::query_as(
            r#"
            INSERT INTO users (email, fullname, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, fullname, email, created_at
            "#,
        )
        .bind(email)
        .bind(fullname)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;
        Ok(user)
    }

    // Equivalents (For Clarity)
    // Because of type inference, these three ways of writing the code are exactly identical to the compiler:

    // let user = sqlx::query_as("...").fetch_one(pool).await?;
    // Ok(user) // Compiler deduces `user` is `User`

    // let user: User = sqlx::query_as("...").fetch_one(pool).await?;
    // let user = sqlx::query_as::<_, User>("...").fetch_one(pool).await?;

    // .bind()
    // In sqlx, the .bind() method is used to safely attach data to the placeholders (like $1, $2, $3 in PostgreSQL) in your SQL query.

    /// Verify email and password
    // find_by_email and verify: These return Result<Option<Self>, AppError> because it's completely normal for a user not to exist (e.g., searching for an unregistered email or entering a bad password). Finding nothing is not a system error; it's a valid empty result (None).
    // mem::take and the mem module
    // std::mem module: A Rust standard library module used for interacting with memory, such as checking sizes, swapping variables, or dropping values.
    // mem::take works by extracting the value out of the mutable reference and leaving the type's default value in its place. Since the field is an Option<String>, its default value is None. This allows you to take ownership of the password hash string without cloning it, while ensuring that the user struct remains in a valid state (with password_hash set to None) after the operation. This is a common pattern for handling sensitive data like password hashes, as it minimizes the time they are kept in memory.
    // After this line executes, there are two different values to consider:
    // The new local variable (password_hash): This gets the actual value that was fetched from the database. Because the query explicitly selected the password_hash column, this variable will be Some("the_hashed_string...") (assuming a hash existed for that user).
    // The struct field (user.password_hash): This becomes None. This is because mem::take replaces the original value with the default for that type. Since password_hash is an Option<String>, its default is None. This means that after mem::take, the user struct no longer holds the password hash, which is a good security practice to minimize the time sensitive data is kept in memory.
    // This is a clever security and optimization pattern: it gives you ownership of the password hash string so you can verify it, but it automatically strips the hash out of the user struct so it doesn't accidentally get leaked when you return the User object (which now has a None in its password field).
    pub async fn verify(
        email: &str,
        password: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, fullname, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;
        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let is_valid = verify_password(password, &password_hash.unwrap_or_default())?; // is a method call on the Option<String> enum in Rust. The unwrap_or_default() method is used to handle the case where password_hash might be None. If password_hash is Some(String), unwrap_or_default() will return the contained String value. If password_hash is None, unwrap_or_default() will return the default value for the type, which for String is an empty string (""). This ensures that the verify_password function always receives a valid string input, even if the original password_hash was not present in the database.
                if is_valid {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

// Password Management Helpers
// These functions use the argon2 crate to handle cryptographic operations:

// hash_password: Generates a secure random salt and hashes a plain-text password using the Argon2id algorithm. It returns the resulting PHC string (e.g., $argon2id$v=19$...).
// verify_password: Takes a plain-text password and a previously generated Argon2 hash string, verifying if they mathematically match.

// Mechanism of Argon2
// Argon2 is a secure password-hashing algorithm designed to resist brute-force and GPU hardware attacks.

// Salt: Random data added to a password before hashing. It ensures that two users with the same password have completely different hashes, neutralizing "rainbow table" (precomputed hash) attacks.
// OsRng: Short for Operating System Random Number Generator. It provides cryptographically secure randomness generated by the OS (used here to generate the salt).
// SaltString: A formatted string representation of the random salt, usually Base64-encoded, making it safe to store in a database.
// Argon2: The struct/configuration runner that applies the Argon2 algorithm (specifically Argon2id) using memory, iterations, and parallelism parameters to produce the final hash.

// Base64 is a binary-to-text encoding scheme used to represent binary data in an ASCII string format.

// It uses 64 characters (A-Z, a-z, 0-9, +, and /, along with = for padding) to translate raw bytes into a safe, easily readable string.

// Why it matters in your code's context (Argon2 / Passwords):
// Cryptographic operations (like generating a random salt or hashing a password) output raw binary data (bytes). If you try to store raw bytes directly in a standard database text column or send them over JSON, it can cause encoding errors or data corruption.

// By encoding the raw salt and hash into Base64, argon2 converts them into safe, printable strings (e.g., $argon2id$v=19$m=...$c2FsdH...) that can be easily stored in your PostgreSQL database as a standard String. This ensures that the password hashes and salts are preserved accurately without any encoding issues, while still being human-readable for debugging purposes.

// What the Salt Looks Like
// A SaltString generates a random sequence of bytes and encodes them using B64 (a variant of Base64 used in password hashing). It typically looks like a 22-character alphanumeric string.

// Example: c2FsdHN0cmluZ3NhbHQ

// What argon2.hash_password Does with It
// When you pass the &salt (along with the plaintext password bytes) into argon2.hash_password(), the following happens under the hood:

// Decodes: The Argon2 library decodes the Base64 SaltString back into raw binary bytes.
// Hashes: It mathematically combines these raw salt bytes with the password bytes using the Argon2id cryptographic algorithm to generate a raw binary hash.
// Formats: It takes the configuration parameters (memory cost, iterations, parallelism), the Base64-encoded salt, and the newly generated Base64-encoded hash, and combines them all into a single PHC (Password Hashing Competition) formatted string.
// The final result looks like this:
// $argon2id$v=19$m=19456,t=2,p=1$c2FsdHN0cmluZ3NhbHQ$WMy...[rest of hash]...

// Argon2 takes these configuration parameters to purposefully make the hashing process expensive and difficult for attackers, specifically those using specialized hardware like GPUs or ASICs to guess passwords.

// Memory Cost: Forces the algorithm to consume a large amount of RAM. GPUs and ASICs have massive processing power but typically lack large amounts of memory per core. Forcing high memory usage makes hardware-based brute-force attacks practically and economically unfeasible.
// Iterations (Time Cost): Determines how many passes the algorithm makes over the memory. This increases the CPU time required to calculate a single hash, slowing down dictionary and brute-force attacks.
// Parallelism: Determines how many independent threads can compute the hash simultaneously. This allows the defender (your server) to compute the hash reasonably fast using multiple multi-core CPUs, while still keeping the overall time and memory requirements extremely high for an attacker.
// By adjusting these parameters, developers can "tune" the hashing process to be exactly as slow and memory-intensive as their specific server infrastructure can tolerate, maximizing security.

// i don't understand, when hacking the remote server, the hacker need to make a lot of cost on his/her attacking computers of memory cost, time cost and Parallelism?

// Yes, that is exactly correct.

// Typically, a hacker does not guess passwords against your live server (where rate-limiting or account lockouts would quickly block them). Instead, they exploit a vulnerability to steal your database, which contains the password hashes, and then try to crack them offline on their own hardware.

// To see if a guessed password (e.g., "password123") matches a stolen hash, the hacker's computer must run the exact same Argon2 algorithm using the identical memory, time, and parallelism parameters embedded in the hash string (e.g., m=19456,t=2,p=1).

// By making the process moderately expensive for your server to compute once during a normal user login, you force the hacker to pay that exact same heavy computational cost for every single guess they make across millions of attempts. This strips away their hardware advantage, making brute-force attacks incredibly slow and economically unfeasible.

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

// PasswordHash
// is a struct from the password-hash crate (which is bundled and re-exported by the argon2 crate). It is used to parse and hold the components of a Password Hashing Competition (PHC) formatted string.

// Here is a deeper look into what it does in your code:

// 1. Parsing the PHC String
// When you retrieve the hashed password string from your database (e.g., $argon2id$v=19$m=19456,t=2,p=1$c2FsdHN0...$WMy...) and pass it to PasswordHash::new(), it dissects this single string into its constituent parts:

// Algorithm Identifier: (argon2id)
// Version: (v=19)
// Parameters: Memory cost (m), iterations/time (t), and parallelism (p).
// Salt: The Base64-encoded salt data.
// Hash Output: The actual Base64-encoded cryptographic hash of the password.
// 2. Preparing for Verification
// The verify_password function requires all of these original ingredients to check if a new login attempt is valid. By parsing the string into a PasswordHash struct, you transform a raw String into a strongly-typed, validated object.

// When you call argon2.verify_password(password.as_bytes(), &password_hash), the library reads the parameters and salt directly from the PasswordHash struct, re-runs the Argon2 algorithm against the plaintext password you provided, and compares the resulting bytes against the hash bytes stored inside the PasswordHash struct.

// password: The plain-text string inputted by the user attempting to log in.
// password_hash: The hashed string retrieved from the database for the corresponding user.

// argon2.verify_password(password.as_bytes(), &password_hash)
// The library then spins up its mathematical engine. It takes the first argument (your raw plain-text password.as_bytes()) and hashes it again.
// Critically, it uses the exact parameters and the exact Salt it extracted from the expected_hash struct in Step 1.
// The two arguments represent the two halves of the verification equation:
// password.as_bytes() is the dynamic input (what the user typed in).
// &password_hash is both the blueprint (how to hash the input) and the answer key (what the final output should look like).

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;

    // Verify password
    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();

    Ok(is_valid)
}

// Testing Definitions
// Unit Tests: Test a small, isolated piece of code (like a single function). For example, testing hash_password without needing a database.
// Integration Tests: Test how multiple pieces of your system work together. For example, the create function requires the Rust logic, the SQL driver, and a live database working in tandem.
// "Spin up" a temporary database: Programmatically creating, starting, and connecting to a completely fresh, isolated database just for the duration of the test, and destroying it afterward. This prevents test data from polluting your real development database.

// #[cfg(test)]
// cfg: Stands for "configuration". It is a conditional compilation attribute.
// test: A specific configuration flag that is only turned on when you run the cargo test command.
// Together, #[cfg(test)] tells the Rust compiler: "Only compile and include this module if we are currently running tests. Do not include this code in the final production binary."

// Does the test logic need to be in the mod tests block?
// It is not strictly required by the compiler (you could just put #[test] functions directly in the file), but it is the standard best practice in Rust. By placing tests inside a mod tests block annotated with #[cfg(test)], you ensure that the test code and test-specific dependencies are only compiled during testing (cargo test). This keeps your production binary small and optimized.

// Why does the mod inside need to import dependencies again?
// In Rust, every module (including inline modules like mod tests) creates a completely new, isolated scope. It does not automatically inherit imports or definitions from its parent file.

// use super::*;: This imports everything from the parent file (the rest of user.rs) into the test module's scope so you have access to User, hash_password, etc., without typing super::User every time.
// Test-specific imports: Crate items like sqlx_db_tester::TestPg and anyhow::Result are only used for writing tests. Importing them exclusively inside the tests module ensures they don't pollute the main application's scope and aren't compiled into the final release build.

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use sqlx_db_tester::TestPg;
    use std::path::Path;

    #[test]
    fn hash_password_and_verify_should_work() -> Result<()> {
        let password = "hunter42";
        let password_hash = hash_password(password)?;
        assert_eq!(password_hash.len(), 97);
        assert!(verify_password(password, &password_hash)?);
        Ok(())
    }

    // verify_password(password, &password_hash)?
    // Unwraps Success: If verify_password succeeds (returns Ok(true) or Ok(false)), the ? operator extracts the boolean value from the Ok and passes it to assert!.
    // Propagates Errors: If verify_password fails (e.g., an issue parsing the hash, returning an Err), the ? operator immediately halts the current function and returns the error. This causes the test function to fail with the specific AppError rather than crashing unexpectedly.

    // assert_eq!, assert!
    // Both are macros used in Rust testing to verify that your code behaves as expected, but they serve slightly different purposes:

    // assert!: Tests a boolean condition. It panics (and fails the test) if the expression inside it evaluates to false.
    // Example: assert!(user.is_some()); fails if the user is None.
    // assert_eq!: Tests for equality between two values. It panics if the left value does not equal the right value. The main advantage of assert_eq! is that if it fails, it prints both the expected and actual values to the console, making debugging much easier.
    // Example: assert_eq!(user.email, email); fails if the user.email does not precisely match the provided email.

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;

        let email = "github@example.com";
        let name = "Kevin Jordan";
        let password = "hunter42";
        let user = User::create(email, name, password, &pool).await?;
        assert_eq!(user.email, email);
        assert_eq!(user.fullname, name);
        assert!(user.id > 0);

        let user = User::find_by_email(email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.fullname, name);

        let user = User::verify(email, password, &pool).await?;
        assert!(user.is_some());
        // The is_some() method simply checks which variant you have. It returns true if the value is Some, and false if the value is None.

        Ok(())
    }
}

// TestPg::new (Initialization & Migrations)
// Creates a completely unique, random database name (e.g., test_db_a1b2c3).
// Connects to your base Postgres server using the provided URL.
// Tells the database engine to create this newly named temporary database.
// Automatically runs all the SQL files located in your ../migrations folder against this new database so that your tables (like users) are perfectly set up before the test even starts.

// Automatic Cleanup (Tear Down)
// Though not explicitly written as a function call, TestPg implements Rust's Drop trait.
// What it does:

// When your test function finishes and the tdb variable goes out of scope, Rust automatically drops the TestPg instance.
// The Drop implementation connects back to the main server and totally drops (deletes) the temporary database, completely cleaning up the environment. This ensures your tests never accidentally share data or pollute your local development database.

// tdb.get_pool().await;
// Returns a standard sqlx::PgPool struct connected specifically to the isolated temporary database created in the previous step.
// You can pass this pool into your application functions (like User::create or User::find_by_email) exactly as you would in production.

// Both #[test] and #[tokio::test] are attribute macros used to define unit tests in Rust, but they are used for different types of functions:

// #[test]
// Use Case: Used for synchronous functions.
// How it works: It is built into the Rust standard library. It simply tells the Rust test runner to execute the function. It does not know how to handle async code or .await keywords.
// Example in your code: hash_password_and_verify_should_work uses basic, synchronous cryptographic functions that do not require an async runtime.
// #[tokio::test]
// Use Case: Used for asynchronous functions (async fn).
// How it works: It is provided by the tokio crate (a popular asynchronous runtime for Rust). Because standard Rust tests cannot run async code directly, #[tokio::test] spins up a hidden Tokio runtime (an event loop) specifically for that test. This allows you to use .await inside the test body.
// Example in your code: create_and_verify_user_should_work is async because it needs to await database connections and queries (pool.get_pool().await, User::create(...).await).

// sqlx, the Rust SQL toolkit, provides a powerful and ergonomic way to interact with databases. One of its core components is the **`Pool`** struct, which is essential for managing database connections efficiently in an asynchronous environment.
// PgPool, specifically, is a type alias for `Pool<Postgres>`, meaning it's a connection pool designed to work with PostgreSQL databases.

// In `sqlx`, **`PgPool`** (which is just an alias for **`sqlx::Pool<sqlx::Postgres>`**) is one of the most important structs you'll use when interacting with a PostgreSQL database.

// Here is a breakdown of its functionality, internal data structure, and most common methods.

// ### 1. Functionality
// The primary purpose of a `Pool` is to manage a set of active database connections that can be reused across different concurrent async tasks.

// *   **Performance:** Establishing a new TCP connection and performing the PostgreSQL authentication handshake for every single query is very slow. A connection pool keeps a set of connections open (idle) and gives them to tasks as needed, drastically reducing overhead.
// *   **Concurrency limits:** The pool enforces a maximum number of connections (e.g., 10 or 50). If your app receives 1,000 requests at once, the pool prevents your app from overwhelming the database by queuing the requests until a connection becomes available.
// *   **Health checking:** `sqlx` automatically tests connections before handing them out and recycles broken connections.

// ### 2. Data Structure
// Under the hood, `Pool<DB>` is essentially a smart pointer wrapped around a shared state.
// *   **Cheap to clone:** It internally uses an `Arc` (Atomic Reference Counted pointer) around its internal state (`PoolInner`). This means you **should** clone the pool to pass it around your app state or into different threads/tasks. You do not need (and shouldn't use) an `Arc<PgPool>`.
// *   **State Management:** It internally uses async communication primitives (like channels or semaphores) to track the idle connections, the active connections, and the tasks waiting for a connection.

// ### 3. Common Methods
// Because `PgPool` is an alias for `Pool<Postgres>`, all methods come directly from the `Pool` struct or traits it implements (like `Executor`).

// #### Creation Methods
// *   **`PgPool::connect(url: &str)`**: The simplest way to create a pool with default settings (usually a max of 10 connections).
// *   **`PgPoolOptions::new()`**: The recommended way to configure a pool before building it.
//     ```rust
//     let pool = PgPoolOptions::new()
//         .max_connections(50)
//         .acquire_timeout(Duration::from_secs(3))
//         .connect("postgres://postgres:password@localhost/mydb")
//         .await?;
//     ```

// #### Connection & Transaction Methods
// *   **`pool.acquire().await`**: Explicitly retrieves a single `PoolConnection<Postgres>` from the pool. You usually don't need this unless you want to hold a connection across multiple manual steps without a transaction.
// *   **`pool.begin().await`**: Retrieves a connection and immediately starts a database transaction (`Transaction<'_, Postgres>`). You must call `.commit().await` or `.rollback().await` on the result.
// *   **`pool.close().await`**: Gracefully shuts down the pool, waiting for running queries to finish.

// #### Executor Trait Methods (Querying)
// `Pool` implements the `Executor` trait, meaning you can pass `&pool` directly into `sqlx` query functions to execute them. When you do this, the pool automatically acquires a connection, runs the query, and instantly returns the connection to the pool.

// ```rust
// // 1. Fetching a single row
// let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", 1)
//     .fetch_one(&pool) // Notice we just pass the pool reference
//     .await?;

// // 2. Fetching multiple rows
// let users = sqlx::query!("SELECT * FROM users")
//     .fetch_all(&pool)
//     .await?;

// // 3. Executing a statement (insert/update/delete)
// let result = sqlx::query!("UPDATE users SET name = $1 WHERE id = $2", "Alice", 1)
//     .execute(&pool)
//     .await?;
// ```

// In `sqlx`, the **`Executor`** trait is one of the most powerful and fundamental design patterns in the library. It acts as a universal adapter allowing you to run queries without worrying about *what* exactly is executing them.

// Here is a breakdown of the `Executor` trait, how `Pool` uses it, and the life cycle of a query like `fetch_one(&pool)`.

// ### 1. The `Executor` Trait

// In SQL databases, you can execute a query in several different contexts:
// 1. On a single, direct TCP database connection (`sqlx::postgres::PgConnection`).
// 2. Inside an active database transaction (`sqlx::Transaction`).
// 3. Through a connection pool (`sqlx::PgPool`).

// To prevent you from having to write different query functions for each of these contexts, `sqlx` provides the **`Executor`** trait.

// Conceptually, the trait looks something like this (simplified):
// ```rust
// pub trait Executor<'c> {
//     type Database: Database;

//     // Executes a query and returns the number of affected rows
//     fn execute<'e, 'q: 'e, E: 'q>(self, query: E) -> ...;

//     // Fetches one row
//     fn fetch_one<'e, 'q: 'e, E: 'q>(self, query: E) -> ...;

//     // ... plus fetch_optional, fetch_all, etc.
// }
// ```

// Any type that implements `Executor` can be passed to the `.fetch_*` or `.execute` methods of a query. In `sqlx`, `Executor` is implemented for:
// * `&mut PgConnection`
// * `&mut Transaction<'_, Postgres>`
// * `&Pool<Postgres>` (which is `&PgPool`)

// ### 2. How `Pool` Implements `Executor`

// When `Executor` is implemented for `&Pool`, it acts as a very convenient wrapper. The pool itself cannot execute a SQL query—only a raw connection can do that.

// So, `Pool`'s implementation of `Executor` essentially says: *"Whenever a query is executed through me, I will automatically grab an idle connection, pass the query to that connection, and then put the connection back."*

// ### 3. What happens during `fetch_one(&pool)`?

// When you build a query and call `.fetch_one(&pool).await`, a very specific sequence of events happens under the hood.

// Let's assume you run:
// ```rust
// let user = sqlx::query_as("SELECT * FROM users WHERE email = $1")
//     .bind(email)
//     .fetch_one(&pool)
//     .await?;
// ```

// Here is the exact step-by-step lifecycle:

// 1. **Query Construction:** `sqlx::query_as` creates a builder object containing your SQL string (`"SELECT *..."`) and your bound parameters (`email`). No database interaction happens yet.
// 2. **Executor Invocation:** You call `.fetch_one(&pool)`, passing a reference to the pool as the `Executor`.
// 3. **Implicit Connection Checkout:** Inside `.fetch_one`, because the executor is a `&Pool`, the pool internally calls `.acquire().await`. This checks out an idle, healthy TCP connection (`PoolConnection`) from the pool's internal state. If no connections are available, this task goes to sleep until one gets returned.
// 4. **Execution:** The TCP connection sends the SQL string and the parameters to the PostgreSQL server.
// 5. **Waiting and Parsing:** The task yields (waits) until the PostgreSQL server responds. Once the data comes back, `sqlx` reads exactly one row from the network stream and maps it to your `User` struct using the `FromRow` trait. (If it finds 0 rows, it generates a `RowNotFound` error).
// 6. **Automatic Connection Release (RAII):** The temporary `PoolConnection` fetched in Step 3 goes out of scope and is dropped. Because it's a smart pointer, its `Drop` implementation automatically returns the underlying TCP connection back to the `PgPool`, marking it as "idle" so the next query can use it.
// 7. **Result Returned:** The function completes, handing you the `Result<User, Error>`.

// ### Summary
// By implementing `Executor` for `&PgPool`, `sqlx` abstracts away the tedious work of writing:
// ```rust
// // You don't have to write this!
// let mut conn = pool.acquire().await?; // Check out
// let user = sqlx::query_as(...).fetch_one(&mut *conn).await?; // Execute
// drop(conn); // Return to pool
// ```
// Instead, `fetch_one(&pool)` does all three steps automatically, safely properly returning the connection to the pool even if the query panics or returns an error.

// sqlx::query_as("SELECT id, fullname, email, created_at FROM users WHERE email = $1")
//     .bind(email)
//     .fetch_optional(pool)
//     .await?;

// You are absolutely right, and that is a great catch. I slightly conflated the two concepts in my simplified explanation.

// You are entirely correct about the standard API usage: when you write `.fetch_optional(&pool)`, you are calling a method on the **`Query`** or **`QueryAs`** struct, **not** the `Executor` trait directly.

// Here is exactly how their relationship works under the hood.

// ### 1. `QueryAs` takes the `Executor` as an argument
// When you chain `.fetch_optional(&pool)`, you are calling this method defined on the `QueryAs` struct:

// ```rust
// impl<'q, O, DB> QueryAs<'q, DB, O, ...> {
//     pub async fn fetch_optional<'e, E>(self, executor: E) -> Result<Option<O>, Error>
//     where
//         E: Executor<'e, Database = DB>,
//     {
//         // ... implementation ...
//     }
// }
// ```
// This is why you pass `&pool` as the argument. The `QueryAs` struct holds your SQL string and parameters, requires you to pass in *something* that implements `Executor` (like `&pool` or `&mut tx`), and then orchestrates the query using that executor.

// ### 2. The `Executor` trait's actual role
// While you don't usually call them directly, the `Executor` trait actually *does* define lower-level versions of these methods. Its true simplified definition looks closer to this:

// ```rust
// pub trait Executor<'c> {
//     type Database: Database;

//     fn fetch_optional<'e, 'q: 'e, E: 'q>(
//         self,
//         query: E, // The query is passed TO the executor here
//     ) -> BoxFuture<'e, Result<Option<<Self::Database as Database>::Row>, Error>>
//     where
//         'c: 'e;

//     // ... fetch_one, execute, etc. ...
// }
// ```

// ### 3. How they connect (The Delegation)
// The `QueryAs` struct's `.fetch_optional(executor)` is simply a convenient, ergonomic wrapper. When you run `query_as(...).fetch_optional(&pool)`, the `QueryAs` struct takes your `&pool`, unpacks its own SQL string and parameters, and internally turns around to call the `Executor` trait's method:

// ```rust
// // Inside sqlx's QueryAs::fetch_optional implementation (conceptually):
// executor.fetch_optional(self.query_string_and_params).await
// ```

// ### Why does `sqlx` design it this way?
// **Ergonomics (Fluent API).** It reads much better top-to-bottom.

// If you had to use the `Executor` trait methods directly, you would have to write this "inside-out" syntax:
// ```rust
// // This is how it works under the hood
// let user = pool.fetch_optional(
//     sqlx::query_as("SELECT ...").bind(email)
// ).await?;
// ```
// Instead, the `QueryAs` methods allow you to elegantly chain the builder pattern:
// ```rust
// // This is the ergonomic API you actually write
// let user = sqlx::query_as("SELECT ...")
//     .bind(email)
//     .fetch_optional(&pool)
//     .await?;
// ```

// So you are 100% correct: the `.fetch_optional` you type is a method on `QueryAs`, which asks for an `Executor` (like `&pool`) to do the heavy lifting of actually running the query.

// argon2.verify_password(password.as_bytes(), &password_hash)

// Yes, `PasswordHash::new(password_hash)` returns `Result<PasswordHash, Error>`. Once unwrapped with `?`, it gives you a **strongly-typed struct** (`PasswordHash`) representing all the individual components of the hash string.

// Here is exactly how `argon2.verify_password()` handles those two seemingly different arguments under the hood.

// ### The Function Signature
// In the `argon2` crate, the `verify_password` method actually comes from a trait named `PasswordVerifier`. Its signature looks roughly like this:

// ```rust
// pub trait PasswordVerifier {
//     fn verify_password(
//         &self,
//         password: &[u8],            // The raw bytes of your plain-text password
//         expected_hash: &PasswordHash // The structured hash from the database
//     ) -> Result<(), Error>;
// }
// ```

// ### What Happens Inside `verify_password`?

// When you call `argon2.verify_password(password.as_bytes(), &password_hash)`, the Argon2 library performs the following logical steps:

// #### Step 1: Read the Parameters
// It looks inside the `&password_hash` struct you provided. This struct contains the specific configuration used to generate this exact hash years ago (or whenever it was created).
// It reads:
// *   The memory cost ($m$)
// *   The time/iterations cost ($t$)
// *   The degree of parallelism ($p$)
// *   The original Salt bytes ($s$)

// #### Step 2: Re-run the Hashing Algorithm
// The library then spins up its mathematical engine. It takes the **first argument** (your raw plain-text `password.as_bytes()`) and hashes it again.
// Critically, it uses the exact parameters and the exact Salt it extracted from the `expected_hash` struct in Step 1.

// #### Step 3: Constant-Time Comparison
// Now the library has two things:
// 1.  **The Expected Output:** The final binary hash extracted from the `&password_hash` struct.
// 2.  **The Computed Output:** The binary hash it just computed in Step 2.

// It compares these two byte arrays.
// *(Note: It uses a special "constant-time" comparison function so that hackers cannot use "timing attacks" to guess the password byte-by-byte).*

// #### Step 4: The Result
// *   If the bytes match perfectly, it returns `Ok(())`.
// *   If they do not match, it returns an `Err(Error::Password)`.

// ### Summary
// The two arguments represent the two halves of the verification equation:
// *   `password.as_bytes()` is the **dynamic input** (what the user typed in).
// *   `&password_hash` is both the **blueprint** (how to hash the input) and the **answer key** (what the final output should look like).
