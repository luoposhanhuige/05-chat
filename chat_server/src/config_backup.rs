use std::{env, fs::File};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // read from  ./app.yml, or /etc/config/app.yml, or from env CHAT_CONFIG
        let ret = match (
            File::open("app.yml"),
            File::open("/etc/config/app.yml"),
            env::var("CHAT_CONFIG"),
        ) {
            (Ok(reader), _, _) => serde_yaml::from_reader(reader),
            (_, Ok(reader), _) => serde_yaml::from_reader(reader),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => bail!("Config file not found"),
        };
        Ok(ret?) // since the type of ret is Result<AppConfig, serde_yaml::Error>, the ? operator will either return the AppConfig if it's Ok, or propagate the error if it's Err.
    }
}

// In Rust, the ? operator is used for error propagation.

// Here is why it is used in Ok(ret?):

// ret is a Result: The function serde_yaml::from_reader(...) returns a Result<AppConfig, serde_yaml::Error>. So the variable ret contains either safely parsed data (Ok) or a parsing error (Err).
// The ? unwraps or returns early:
// If ret is an Err, the ? operator immediately halts the function and returns that error back to the caller (automatically converting it into an anyhow::Error).
// If ret is an Ok, the ? operator extracts the inner AppConfig value.
// Ok(...) wraps it back: The extracted AppConfig value is then wrapped back into a new Ok() to match the function's return signature, which is anyhow::Result<Self>.

// Here is how the compiler figures it out:

// The Return Signature: The function is defined as pub fn load() -> Result<Self>. Because this is inside an impl AppConfig block, Self means AppConfig. Therefore, the function must return a Result<AppConfig, anyhow::Error>.
// The Final Statement: The last line of the function is Ok(ret?). For this to match the return signature, whatever ret? evaluates to must be an AppConfig.
// Working Backwards (Type Inference):
// ret is the result of serde_yaml::from_reader(reader).
// As shown in your screenshot, from_reader is a generic function: fn from_reader<R, T>(rdr: R) -> Result<T>.
// Because the compiler knows ret? needs to be an AppConfig, it automatically infers that the generic type T must be AppConfig.

// Here is exactly what happens inside `serde_yaml::from_reader` once the compiler knows `T` is `AppConfig`:

// ### 1. The file is *already* open
// `from_reader` does not open the file. You already opened it with `File::open(...)` and passed the resulting handle (`rdr`) into the function. The generic `R` is inferred as `std::fs::File`. Because `File` implements the `std::io::Read` trait, `from_reader` can read bytes from it.

// ### 2. Reading and Parsing Bytes
// Inside `from_reader`, the function reads the raw bytes from the file stream until the end. It then parses those raw bytes according to the YAML specification, turning them into an intermediate internal representation (like a dictionary/map of keys and values).

// ### 3. The Magic: `Deserialize`
// Because `from_reader` requires `T` to implement the `DeserializeOwned` trait (as seen in your previous screenshot), it looks at `AppConfig`.

// In your code, you have:
// ```rust
// #[derive(Debug, Serialize, Deserialize)]
// pub struct AppConfig {
//     pub server: ServerConfig,
// }
// ```
// The `#[derive(Deserialize)]` macro automatically generated hidden Rust code that tells Serde exactly how to build an `AppConfig`.

// `from_reader` essentially asks this generated code: *"I found a YAML structure. Can you build an `AppConfig` from it?"*

// ### 4. Field Mapping
// The generated `Deserialize` code takes over:
// * It looks for a key named `"server"` in the parsed YAML.
// * If it finds it, it sees that `server` expects a `ServerConfig`.
// * It recursively looks at the `ServerConfig` definition (which also has `#[derive(Deserialize)]`) and looks for a `"port"` key that holds an integer (`u16`).
// * If all fields match the expected types and names in the YAML file, it constructs the structs in memory.

// ### 5. Returning the Result
// * If everything maps perfectly, it wraps the newly created `AppConfig` instance in `Ok(AppConfig)` and returns it.
// * If a field is missing, a type is wrong (e.g., `"port"` is a string instead of a number), or the YAML relies on invalid syntax, it stops and returns an `Err(serde_yaml::Error)`.

// File::open("app.yml")

// works because relative file paths in Rust are evaluated based on the Current Working Directory (CWD) of the running process at runtime, not the location of the source code file (config.rs) at compile time.

// When you run a Rust project using cargo run (or run the compiled binary directly from the project root), the Current Working Directory is set to the root of the crate—the folder containing your Cargo.toml.

// Looking at your screenshot:

// Cargo.toml is in the chat_server folder.
// app.yml is also in the chat_server folder.
// Therefore, when the program runs, it looks for app.yml in its current working directory (.../chat/chat_server/), finds it successfully, and opens it, completely agnostic to the fact that the code calling it is inside the src/ directory.
