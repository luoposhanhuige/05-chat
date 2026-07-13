// Grant the postgres user the correct privileges in your local database.
// psql -U xhui -d postgres -c "ALTER USER postgres CREATEDB;"
//          btw, I ran a command to check the existing users by explicitly connecting to the default postgres database (psql -U xhui -d postgres -c "\du").
// cargo test

use crate::{AppError, User};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
// PasswordHasher - Trait for Creating Hashes
//  fn hash_password
// Argon2 implements this trait

// PasswordHash - Represents the Output Hash

// PasswordVerifier - Trait for Verifying Passwords
// Argon2 implements this trait

// pub struct PasswordHash {
// Internally stores:
//     algorithm: String,         // "argon2id"
//     version: u32,              // 19
//     params: ParamsString,      // "m=19456,t=2,p=1"
//     salt: SaltString,          // "j7DfSx9nK2pQ4wL8mR"
//     hash: Vec<u8>,             // The actual hash bytes
// }

// String representation:
// "$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$a9B3c5D7e9F1g3H5i7J9"
//  ^algorithm  ^version           ^salt                 ^hash

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::mem;

// Summary
// create: Used exclusively in your POST /register route.
// verify: Used exclusively in your POST /login route.
// find_by_email: Used as a utility tool across many routes (validating the sign-up form dynamically, password resets, or checking user permissions before performing an action).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    // RegisterRequest. Input struct for user registration
    pub fullname: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigninUser {
    // LoginRequest. Input struct for user login
    pub email: String,
    pub password: String,
}

// Registration flow
// CreateUser {
//     fullname: "Tyr Chen",      // NEW user data
//     email: "tchen@acme.org",   // NEW account
//     password: "hunter42",      // NEW password to set
// }

// Login flow
// SigninUser {
//     email: "tchen@acme.org",   // EXISTING account lookup
//     password: "hunter42",      // PASSWORD verification
// }

impl User {
    /// Find a user by email
    // Executes a SELECT query to fetch a user by their email address. It returns Ok(Option<User>), successfully handling cases where the user does not exist.
    // fetch_optional(pool): Expects zero or one row. If a row is found, it returns Ok(Some(User)). If no row is found, it returns Ok(None). It only returns an error if something actually goes wrong (like a syntax error or a broken connection). This is appropriate here because it's perfectly valid for a user to not exist with the given email, and we want to handle that case gracefully without treating it as an error.
    // find_by_email and verify: These return Result<Option<Self>, AppError> because it's completely normal for a user not to exist (e.g., searching for an unregistered email or entering a bad password). Finding nothing is not a system error; it's a valid empty result (None).
    pub(crate) async fn find_by_email(
        email: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>, AppError> {
        let user =
            sqlx::query_as("SELECT id, fullname, email, created_at FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(pool)
                .await?;
        Ok(user)
    }

    // for registration
    // Create a new user
    // Hashes the provided plain-text password, then executes an INSERT statement to add a new row to the database. It returns the newly created User.
    // fetch_one(pool): Expects exactly one row. If a row is found, it returns Ok(User). If no row is found, it returns an Error. This is appropriate here because the RETURNING clause guarantees that a row will be returned if the insert is successful. If something goes wrong (like a constraint violation), it will return an error.
    // create: This returns Result<Self, AppError> because an INSERT statement is expected to create exactly one record. If it succeeds, you definitely get a User back. If it fails (e.g., the email already exists, triggering a database constraint), it returns a system error (AppError).
    pub(crate) async fn create(input: &CreateUser, pool: &PgPool) -> Result<Self, AppError> {
        let password_hash = hash_password(&input.password)?;
        // check if email exists
        let user = Self::find_by_email(&input.email, pool).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }
        let user = sqlx::query_as(
            r#"
            INSERT INTO users (email, fullname, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, fullname, email, created_at
            "#,
        )
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;
        Ok(user)
    }

    // for login
    // Verify email and password
    // find_by_email and verify: These return Result<Option<Self>, AppError> because it's completely normal for a user not to exist (e.g., searching for an unregistered email or entering a bad password). Finding nothing is not a system error; it's a valid empty result (None).
    // mem::take and the mem module
    // std::mem module: A Rust standard library module used for interacting with memory, such as checking sizes, swapping variables, or dropping values.
    // mem::take works by extracting the value out of the mutable reference and leaving the type's default value in its place. Since the field is an Option<String>, its default value is None. This allows you to take ownership of the password hash string without cloning it, while ensuring that the user struct remains in a valid state (with password_hash set to None) after the operation. This is a common pattern for handling sensitive data like password hashes, as it minimizes the time they are kept in memory.
    // After this line executes, there are two different values to consider:
    // The new local variable (password_hash): This gets the actual value that was fetched from the database. Because the query explicitly selected the password_hash column, this variable will be Some("the_hashed_string...") (assuming a hash existed for that user).
    // The struct field (user.password_hash): This becomes None. This is because mem::take replaces the original value with the default for that type. Since password_hash is an Option<String>, its default is None. This means that after mem::take, the user struct no longer holds the password hash, which is a good security practice to minimize the time sensitive data is kept in memory.
    // This is a clever security and optimization pattern: it gives you ownership of the password hash string so you can verify it, but it automatically strips the hash out of the user struct so it doesn't accidentally get leaked when you return the User object (which now has a None in its password field).
    pub(crate) async fn verify(
        input: &SigninUser,
        pool: &PgPool,
    ) -> Result<Option<Self>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, fullname, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&input.email)
        .fetch_optional(pool)
        .await?;
        match user {
            // user_opt
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let is_valid =
                    verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if is_valid {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
                // Rewritten: if-else becomes match
                // match is_valid {
                //     true => Ok(Some(user)),
                //     false => Ok(None),
                // }
            }
            None => Ok(None),
        }
    }
}

// What unwrap_or_default() Does
// let password_hash = mem::take(&mut user.password_hash);
//                  ↑ password_hash is Option<String>

// let is_valid = verify_password(&input.password,
//                                &password_hash.unwrap_or_default())?;
//                                            ↑ Converts Option<String> to &str

// If password_hash is Some("hash"), extract the string
// If password_hash is None, use default (empty string "")

// BEFORE mem::take():
// user.password_hash = Some("$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$...")

// AFTER mem::take():
// user.password_hash = None
// password_hash = Some("$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$...")

// 1. Query database for user with email
//    ↓
// 2. Get back: User {
//      id: 1,
//      email: "tchen@acme.org",
//      password_hash: Some("$argon2id$..."),  ← Sensitive data!
//    }
//    ↓
// 3. mem::take() extracts the hash
//    ↓
//    user.password_hash = None  ← Automatically cleared!
//    password_hash = Some("$argon2id$...")
//    ↓
// 4. Use password_hash to verify the password
//    ↓
// 5. Return Ok(Some(user))

//    Now user has:
//    {
//      id: 1,
//      email: "tchen@acme.org",
//      password_hash: None,  ← Hash is gone!
//    }
//    ↓
// 6. If user gets sent to client or logged, password_hash is not exposed

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    //                                                    ↑ OsRng (0 bytes)
    //         Returns SaltString (~24-40 bytes)
    //         Example: "j7DfSx9nK2pQ4wL8mR" (16 random bytes, Base64-encoded)

    // Argon2 with default params (Argon2id v19)
    // Argon2 struct implements PasswordHasher trait which has the method hash_password
    let argon2 = Argon2::default();
    //  ↑ Creates an Argon2 instance with DEFAULT parameters
    // Argon2::default() creates:
    // Argon2 {
    //     algorithm: Argon2id,      // Argon2id variant (hybrid approach)
    //     version: Version19,       // Version 19 (latest)
    //     params: Params {          // Default parameters
    //         memory_cost: 19456,   // 19 MB
    //         time_cost: 2,         // 2 iterations
    //         parallelism: 1,       // 1 thread
    //         ...
    //     },
    // }

    // It's just a CONFIGURATION HOLDER:

    // ✓ Doesn't hash anything yet
    // ✓ Just stores settings (memory, time, parallelism)
    // ✓ Implements PasswordHasher trait (the actual hashing method)
    // ✓ Implements PasswordVerifier trait (the verification method)

    // Hash password to PHC string ($argon2id$v=19$...)
    // what does PHC mean? Password Hashing Competition, a standard format for encoding password hashes that includes all necessary information (algorithm, version, parameters, salt, hash) in a single string. This makes it easy to store and verify password hashes without needing to manage separate fields for salt and parameters.
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    // password.as_bytes()
    // Converts: &str (text string) → &[u8] (byte array)
    // Why? Because cryptographic functions work with raw bytes, not text strings.
    // Cryptographic algorithms operate on bit patterns, so they require data in a binary format (byte arrays) rather than human-readable text. The .as_bytes() method converts the password string into a byte slice, which can then be processed by the hashing function.
    // hash_password returns a PasswordHash struct, which implements the ToString trait. Calling .to_string() converts it into the standard PHC string format that includes all the hashing parameters and the resulting hash, making it easy to store in the database and verify later.

    Ok(password_hash)
}

// Creation:
// let salt = SaltString::generate(&mut OsRng);
//          ↑ Static method that generates random salt
//                             ↑ Uses OsRng for randomness

// What gets generated:
// 16 random bytes → Base64-encoded → SaltString
// Example internal value: "j7DfSx9nK2pQ4wL8mR"

// Same password, different salts:

// Password: "hunter42"
// Salt 1: "j7DfSx9nK2pQ4wL8mR"
//   → Result: "$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$a9B3c5D7e9F1g3H5i7J9"

// Same password: "hunter42"
// Salt 2: "aB1cD2eF3gH4iJ5kL6m"
//   → Result: "$argon2id$v=19$m=19456,t=2,p=1$aB1cD2eF3gH4iJ5kL6m$x9Y8z7W6v5U4t3S2r1Q0"

// Different hashes even though password is same!
// This prevents rainbow table attacks.

// PasswordHasher - Trait for Creating Hashes
// Returns PasswordHash struct
// pub trait PasswordHasher {
//     fn hash_password(
//         &self,
//         password: &[u8],
//         salt: &SaltString,
//     ) -> Result<PasswordHash, Error>;
// }
// Argon2 implements this trait

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    // Argon2 struct implements PasswordVerifier trait which has the method verify_password
    let argon2 = Argon2::default();

    // Parse the stored hash string back to PasswordHash struct
    let password_hash = PasswordHash::new(password_hash)?;
    //  ↑ Parses: "$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$a9B3c5D7e9F1g3H5i7J9"
    //  ↑ Extracts: algorithm, version, params, salt, stored_hash

    // Verify password
    // Call PasswordVerifier trait method
    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();

    Ok(is_valid)
}

// What does .verify_password() return?
// Result<(), PasswordHashError>
//   ├─ Ok(())      ← Password is CORRECT
//   └─ Err(...)    ← Password is WRONG (or error)

// With .is_ok():
// let is_valid = result.is_ok();
// Ok(()) → true (password matches)
// Err(...) → false (password doesn't match)
// Returns: bool

// Why Not Use ? Instead of .is_ok()?
// With ?:
// result?;
// Ok(()) → Unwraps to (), continues execution
// Err(...) → Returns error immediately (early exit)
// Returns: nothing (but continues or exits)

// What verify_password does internally:
// Input password: "hunter42"
// Stored hash: "$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$a9B3c5D7e9F1g3H5i7J9"

// Step 1: Extract from stored hash
//   - Algorithm: argon2id
//   - Version: 19
//   - Memory: 19456 KiB
//   - Time: 2 iterations
//   - Parallelism: 1
//   - Salt: j7DfSx9nK2pQ4wL8mR
//   - Stored hash: a9B3c5D7e9F1g3H5i7J9

// Step 2: Re-hash incoming password with SAME parameters and SAME salt
//   Argon2("hunter42", salt="j7DfSx9nK2pQ4wL8mR", m=19456, t=2, p=1)
//   → Computed hash: "a9B3c5D7e9F1g3H5i7J9"

// Step 3: Compare
//   Computed hash == Stored hash?
//   "a9B3c5D7e9F1g3H5i7J9" == "a9B3c5D7e9F1g3H5i7J9" ✓ Match!
//   → Return Ok(())

// If password was wrong:
//   Argon2("wrongpassword", ...)
//   → Computed hash: "xyz123abc..."
//   → "xyz123abc..." != "a9B3c5D7e9F1g3H5i7J9" ✗ No match
//   → Return Err(...)

#[cfg(test)]
// In tests, we need to CREATE User objects WITHOUT a database.
// The create() method requires a database connection, which we don't want in unit tests. So we add a simple constructor for testing purposes that allows us to create User instances directly with specified values.
impl User {
    pub fn new(id: i64, fullname: &str, email: &str) -> Self {
        Self {
            id,
            fullname: fullname.to_string(),
            email: email.to_string(),
            password_hash: None,
            created_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
impl CreateUser {
    pub fn new(fullname: &str, email: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // import all the structs and functions from the parent module, in this case, the user.rs file. This allows us to use User, CreateUser, SigninUser, hash_password, verify_password, and any other items defined in the parent module without needing to prefix them with the module name.
    use anyhow::Result;
    use sqlx_db_tester::TestPg;
    use std::path::Path;

    #[test]
    fn hash_password_and_verify_should_work() -> Result<()> {
        let password = "hunter42";
        let password_hash = hash_password(password)?;
        //  Returns something like:
        //  "$argon2id$v=19$m=19456,t=2,p=1$j7DfSx9nK2pQ4wL8mR$a9B3c5D7e9F1g3H5i7J9"
        assert_eq!(password_hash.len(), 97);
        assert!(verify_password(password, &password_hash)?);
        // assert!() is a macro used in tests to verify that a condition is true.
        // assert!(condition);
        // ✓ If condition is true → Test passes, continues
        // ✗ If condition is false → Test fails, panics with error message
        Ok(())
    }

    // sqlx_db_tester is a Rust testing utility crate that simplifies database testing by providing fixtures and helpers for managing test databases.
    // TestPg is a test fixture specifically for PostgreSQL databases. It provides:
    // Automatic database creation: Spins up a temporary PostgreSQL database for tests
    // Connection pooling: Manages SQLx connection pools for test isolation
    // Cleanup: Automatically tears down test databases after tests complete
    // Migration support: Can run migrations before tests
    // Transaction rollback: Options to rollback changes after each test
    #[tokio::test]
    async fn create_duplicate_user_should_fail() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;
        // So the pattern is:

        // TestPg::new(...) = Configuration setup (lightweight)
        // tdb.get_pool().await = Actual initialization (creates temp DB, runs migrations, etc.)

        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "hunter42");
        User::create(&input, &pool).await?;
        let ret = User::create(&input, &pool).await;
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => {
                assert_eq!(email, input.email);
            }
            _ => panic!("Expecting EmailAlreadyExists error"),
        }
        Ok(())
    }
    // Line 1 uses ? operator:

    // If it succeeds → continues
    // If it fails → immediately returns the error from the test function
    // The test would fail before reaching line 2
    // Line 2 captures the result without ?:

    // Whether it succeeds or fails, the result is stored in ret
    // You can then pattern-match on it to test the specific error
    // This is intentional! The test does:

    // Line 1: Create a user successfully (first time should work)
    // Line 2: Try creating the same user again (should fail with EmailAlreadyExists)
    // Pattern match on the failure to verify it's the correct error
    // If line 1 had also used let ret = without ?, the test wouldn't propagate the error from the first creation, making the test harder to debug.

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;

        let input = CreateUser::new("Tyr Chen", "tchen@acme.org", "hunter42");
        let user = User::create(&input, &pool).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = User::find_by_email(&input.email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = User::verify(&input, &pool).await?;
        assert!(user.is_some());

        Ok(())
    }
}
