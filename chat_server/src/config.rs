use std::{env, fs::File};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

// AppConfig {
//     server: ServerConfig {
//         port: 6688,
//         db_url: "postgres://postgres:postgres@localhost:5432/chat".to_string(),
//     },
//     auth: AuthConfig {
//         sk: "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n".to_string(),
//         pk: "-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----\n".to_string(),
//     },
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub sk: String,
    pub pk: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
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
        Ok(ret?)
        // 3
        // Because ret is a Result<_, serde_yaml::Error>, not a Result<_, anyhow::Error>.
        // This is equivalent to:
        // let config = ret?;
        // Ok(config)
        // Ok(ret?) is used to convert the error type and still return anyhow::Result<Self>.

        // enum variant names also act as constructors.
        // Ok(...) is the constructor for the Ok variant of the Result enum, which wraps a successful value. In this case, it wraps the AppConfig value extracted from ret?.
    }
}

// Here is why it is used in Ok(ret?):

// ret is a Result: The function serde_yaml::from_reader(...) returns a Result<AppConfig, serde_yaml::Error>. So the variable ret contains either safely parsed data (Ok) or a parsing error (Err).
// The ? unwraps or returns early:
// If ret is an Err, the ? operator immediately halts the function and returns that error back to the caller (automatically converting it into an anyhow::Error).
// If ret is an Ok, the ? operator extracts the inner AppConfig value.
// Ok(...) wraps it back: The extracted AppConfig value is then wrapped back into a new Ok() to match the function's return signature, which is anyhow::Result<Self>.

// 1
// `reader` is the value extracted from:

// ```rust
// File::open("app.yml")
// ```

// More exactly:

// - `File::open("app.yml")` returns `Result<File, std::io::Error>`
// - the pattern `Ok(reader)` means:
//   - if opening succeeds
//   - bind the inner `File` to the variable `reader`

// So here, `reader` is a `std::fs::File`.

// `serde_yaml::from_reader(reader)` accepts it because `File` implements the `Read` trait, so Serde can read YAML data from that file stream.

// ## In short

// - `reader` = the opened `app.yml` file handle
// - type = `std::fs::File`
// - purpose = provide bytes to `serde_yaml::from_reader(...)`

// The name `reader` is just a variable name. It could also have been called `file`.

// 2
// Q: so, the serde_yaml::from_reader infer the AppConfig as the struct to deserialize given that the signature of pub fn load() -> Result<Self>, right?

// A: Yes, that's correct. The `serde_yaml::from_reader` function is a generic function that can deserialize any type that implements the `Deserialize` trait. In this case, because the return type of the `load` function is `Result<Self>`, and `Self` refers to `AppConfig`, the compiler infers that `serde_yaml::from_reader` should deserialize the YAML data into an `AppConfig` struct.
