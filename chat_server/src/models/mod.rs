mod user;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub fullname: String,
    pub email: String,
    #[sqlx(default)]
    // This instructs the sqlx::FromRow macro to fall back to the type's default value if the query does not return a password_hash column. Since the type is Option<String>, the default is None. This is useful because it allows you to query the database for a User without being forced to always select the password_hash column (e.g., when you just need the user's public profile data).
    #[serde(skip)]
    // This instructs the serde crate's Serialize and Deserialize macros to completely ignore this field. When you convert the User struct into JSON (or another format) to send to a client, the password_hash will not be included. This is a crucial security practice to ensure password hashes are never accidentally exposed via an API.
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

// question:
// is it necessary for the mod.rs separates the User definition from it's methods defined in the user.rs? what is design pattern of that?

// answer:
// it is not strictly necessary. In Rust, you can define a struct and its impl block in the exact same file. Doing so is actually the most common approach for smaller types.
// Separating the data definition (mod.rs) from its behavior/methods (user.rs) is an organizational choice rather than a formal gang-of-four design pattern. It aligns with a few software design principles:
// Separation of Concerns: mod.rs defines the raw data structure (an Entity or Data Transfer Object), while user.rs contains the business logic and database operations (acting like a Data Access Object or Repository pattern).
// Module Facade: By defining or re-exporting the models in the mod.rs file, you create a clean public API for the models module. Other parts of the app can simply use crate::models::User; instead of knowing the internal file structure.
// File Size Management: As an application grows, database queries, password hashing, and unit tests can make a file quite large. Moving the implementation to a separate file keeps the module index (mod.rs) easy to read at a glance.
// In summary, while it's not a strict requirement, this organizational pattern can help maintain clarity and manageability as your codebase grows.

// pub email: String,
// If you wanted to enforce email formatting via macro attributes, developers typically use a crate like validator. You would add the crate and then apply an attribute like this:
// Requires the `validator` crate
// #[derive(Validate)]
// pub struct User {
// ...
// #[validate(email)]
//     pub email: String,
// ...
// }

// The best solution is a multi-layered approach, often referred to as **Defense in Depth**, which utilizes both application-level and database-level validation.

// 1. **Application Level (Rust Structs/Methods):**
//    Validating early in the application—either by using the `validator` crate or the "Newtype" pattern (e.g., creating a custom `struct Email(String)` that validates upon instantiation)—is recommended. This provides fast feedback, produces user-friendly error messages, and avoids unnecessary database calls with invalid data.

// 2. **Database Level (SQL Constraints):**
//    Applying a `CHECK` constraint (such as a regex pattern for emails) in the database acts as the final line of defense. This guarantees data integrity, ensuring that invalid emails cannot be inserted even if a bug bypasses application logic or if data is inserted manually via other tools.

// Using both ensures early error detection in the backend while maintaining absolute data integrity in the storage layer.
