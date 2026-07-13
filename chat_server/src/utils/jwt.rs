use crate::{AppError, User};
use jwt_simple::prelude::*;

const JWT_DURATION: u64 = 60 * 60 * 24 * 7;
const JWT_ISS: &str = "chat_server";
const JWT_AUD: &str = "chat_web";

pub struct EncodingKey(Ed25519KeyPair);

#[allow(unused)]
pub struct DecodingKey(Ed25519PublicKey);

impl EncodingKey {
    pub fn load(pem: &str) -> Result<Self, AppError> {
        Ok(Self(Ed25519KeyPair::from_pem(pem)?))
        //       ↑ pem is the PEM STRING from AppConfig.auth.sk
        //       It's a &str, not a file path!
        //           It does NOT read from app.yml file
        //           It does NOT read from AppConfig
        //           It ONLY reads from the pem parameter (the STRING)
        // Ed25519KeyPair::from_pem() parses the PEM string, extracts the private key, and derives the public key to create an Ed25519KeyPair instance.
    }
    // Self (Capital S) - Type Constructor
    // Self is a type alias referring to the struct itself. It's used to construct a new instance
    // in this case, it creates a new EncodingKey instance wrapping the Ed25519KeyPair.

    pub fn sign(&self, user: impl Into<User>) -> Result<String, AppError> {
        let claims = Claims::with_custom_claims(user.into(), Duration::from_secs(JWT_DURATION));
        let claims = claims.with_issuer(JWT_ISS).with_audience(JWT_AUD);
        Ok(self.0.sign(claims)?)
    }
}

// The with_custom_claims() method uses a generic type parameter CustomClaims:

// impl<CustomClaims> Claims<CustomClaims> {
//     pub fn with_custom_claims(custom: CustomClaims, duration: Duration) -> Self {
//         let now = Utc::now().timestamp() as u64;
//         Self {
//             custom,
//             issued_at: now,
//             expires_at: now + duration.as_secs(),
//             issuer: None,
//             audience: None,
//             subject: None,
//             jwt_id: None,
//         }
//     }
// }
// Any type can be CustomClaims - there's no special trait requirement shown in the signature. However, internally, JWTClaims needs to serialize your type to JSON:
// #[derive(Serialize, Deserialize)]
// pub struct JWTClaims<CustomClaims> {
//     pub custom: CustomClaims,
//     pub issued_at: i64,
//     pub expires_at: i64,
//     // ...
// }

// Why User works:
// User derives Serialize + Deserialize ✓, so it can be converted to/from JSON:
// // Step 1: Create Claims
// let claims = Claims::with_custom_claims(user, Duration::from_secs(JWT_DURATION));
// //                                       ↑ User implements Serialize
// //                                       ✓ Fits CustomClaims requirement

// What sign() Returns
// the return value is Base64URL-encoded text, but specifically a JWT token string with 3 parts:
// eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjEsImZ1bGxuYW1lIjoiVHlyIENoZW4iLCJlbWFpbCI6InRjaGVuQGFjbWUub3JnIiwicGFzc3dvcmRfaGFzaCI6bnVsbCwiY3JlYXRlZF9hdCI6IjIwMjQtMDEtMDFUMDAwMDowMFoifSwiaWF0IjoxNjg1MDAwMDAwLCJleHAiOjE2ODU2MDQ4MDAsImlzcyI6ImNoYXRfc2VydmVyIiwiYXVkIjoiY2hhdF93ZWIifQ.X8jK9mL2qP3rS4tU5vW6xY7zA8bC9dE0fG1hI2jJ3k4

// 1
// Full Flow
// User { id: 1, fullname: "Tyr", email: "..." }
//     ↓
// Claims::with_custom_claims(user, 7_days)
//     ↓
// Claims {
//     custom: User { ... },
//     issued_at: 1685000000,
//     expires_at: 1685604800,  // 7 days later
//     issuer: None,
//     audience: None,
//     ...
// }
//     ↓
// .with_issuer("chat_server").with_audience("chat_web")
//     ↓
// Claims {
//     custom: User { ... },
//     issued_at: 1685000000,
//     expires_at: 1685604800,
//     issuer: Some("chat_server"),
//     audience: Some("chat_web"),
//     ...
// }
//     ↓
// self.0.sign(claims)
//     ↓
// JWT Token String: "eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjE..."

// 2
// ## What `self.0.sign(claims)` Does

// ```rust
// Ok(self.0.sign(claims)?)
// //  ↑ Ed25519KeyPair
// //      ↑ sign() method performs Ed25519 digital signature
// ```

// **Step-by-step:**

// 1. **Serializes Claims to JSON**
//    ```
//    Claims { custom: User {...}, issued_at: ..., expires_at: ..., ... }
//        ↓
//    JSON: {"custom":{"id":1,"fullname":"Tyr Chen",...},"issued_at":1685000000,...}
//    ```

// 2. **Creates JWT structure (3 parts separated by dots)**
//    ```
//    JWT = Header.Payload.Signature
//    ```

// 3. **Signs with Ed25519 private key**
//    - Uses the private key to create a cryptographic signature
//    - Returns **64-byte binary signature** (Ed25519 signatures are always 64 bytes)

// 4. **Encodes to Base64URL** (not hexadecimal)
//    ```
//    eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjE...
//    ↑ Header (Base64URL)  ↑ Payload (Base64URL)  ↑ Signature (Base64URL)
//    ```

// ## The Format

// ```
// JWT Token String:
// eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjE...Ky5xk3zA4.X8jK9mL2qP3rS4tU5vW6xY7zA8bC9dE0fG1hI2jJ3k4

// ├─ Header (Base64URL)
// ├─ Payload (Base64URL)
// └─ Signature (Base64URL)
// ```

// ## Not Hexadecimal, But Base64URL

// ```rust
// // Hexadecimal (what you might think)
// // a3f4b2c1d5e6...  (each byte = 2 hex chars)

// // Base64URL (what JWT actually uses)
// // a3f4b2c1d5e6...  (more compact, URL-safe encoding)
// ```

// **Base64URL is preferred for JWTs because:**
// - More compact than hex
// - URL-safe (no special characters that break URLs)
// - Web standard for token transmission

// So the token is a **Base64URL-encoded three-part JWT**, not a hexadecimal representation.

// JWT Signing Process (What ek.sign() does)
// Step 1: Create Header and Payload JSON
// Header: {
//     "alg": "EdDSA",
//     "typ": "JWT"
// }

// Payload: {
//     "custom": {
//         "id": 1,
//         "fullname": "Tyr Chen",
//         "email": "tchen@acme.org"
//     },
//     "issued_at": 1685000000,
//     "expires_at": 1685604800,
//     "issuer": "chat_server",
//     "audience": "chat_web"
// }

// Step 2: Encode BOTH independently to Base64URL
// Encode Header JSON to Base64URL
// Header JSON: {"alg":"EdDSA","typ":"JWT"}
//     ↓
// Base64URL: eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9
//     ↓
// Part 1: eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9

// // Encode Payload JSON to Base64URL (independently)
// Payload JSON: {"custom":{"id":1,...},"issued_at":1685000000,...}
//     ↓
// Base64URL: eyJjdXN0b20iOnsiaWQiOjEsImZ1bGxuYW1lIjoiVHlyIENobiBlbiIsImVtYWlsIjoidGNoZW5AYWNtZS5vcmcifSwiaWF...
//     ↓
// Part 2: eyJjdXN0b20iOnsiaWQiOjEsImZ1bGxuYW1lIjoiVHlyIENobiBlbiIsImVtYWlsIjoidGNoZW5AYWNtZS5vcmcifSwiaWF...

// Step 3: Combine Header.Payload (with dot separator)
// Message to Sign: "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJjdXN0b20iOnsiaWQiOjE..."
//                  ↑ Header (Base64URL)        ↑ Payload (Base64URL)
//                                     ↑ Dot separator (NOT encoded)

// Step 4: Sign the combined string using Ed25519 PRIVATE key
// // Input to Ed25519 signing:
// String to sign: "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJjdXN0b20iOnsiaWQiOjE..."

// // Ed25519 signing process:
// self.0.sign(claims)  // ← self.0 is Ed25519KeyPair containing private key
//     ↓
// Ed25519 private key signs the string
//     ↓
// Produces: 64-byte binary signature

// // Example signature bytes (in binary):
// [0x12, 0x34, 0x56, 0x78, ..., (64 bytes total)]

// Step 5: Encode signature to Base64URL
// 64-byte binary signature: [0x12, 0x34, 0x56, 0x78, ...]
//     ↓
// Base64URL encode: X8jK9mL2qP3rS4tU5vW6xY7zA8bC9dE0fG1hI2jJ3k4
//     ↓
// Part 3: X8jK9mL2qP3rS4tU5vW6xY7zA8bC9dE0fG1hI2jJ3k4

// Step 6: Combine all 3 Base64URL parts with dots
// Final JWT Token:
// eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJjdXN0b20iOnsiaWQiOjE...Ky5xk3zA4.X8jK9mL2qP3rS4tU5vW6xY7zA8bC9dE0fG1hI2jJ3k4
// ↑ Header (Base64URL)                    ↑ Payload (Base64URL)            ↑ Signature (Base64URL)

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, AppError> {
        Ok(Self(Ed25519PublicKey::from_pem(pem)?))
    }

    #[allow(unused)]
    pub fn verify(&self, token: &str) -> Result<User, AppError> {
        let opts = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[JWT_ISS])),
            allowed_audiences: Some(HashSet::from_strings(&[JWT_AUD])),
            ..Default::default()
        };

        let claims = self.0.verify_token::<User>(token, Some(opts))?;
        Ok(claims.custom)
    }
}

// self.0.verify_token::<User>
// Type parameter (turbofish syntax)

// fn verify_token<CustomClaims: DeserializeOwned>(
//     &self,
//     token: &str,
//     options: Option<VerificationOptions>,
// ) -> Result<JWTClaims<CustomClaims>, Error>
// 1. Takes the JWT token string and optional verification options
// 2. Verifies the token's signature using the Ed25519 public key
// 3. Checks the claims against the provided options (issuer, audience, expiration, etc.)
// 4. If valid, deserializes the payload into JWTClaims<CustomClaims>
// 5. Returns the claims, which includes the custom user data

// What verify_token::<T> Actually Returns
// let claims = self.0.verify_token::<User>(token, Some(opts))?;
// //  ↑ This is Claims<User>, NOT just User
// Returns the complete Claims object
// Claims<User> {
//     custom: User { id: 1, fullname: "Tyr", email: "..." },
//     issued_at: 1685000000,
//     expires_at: 1685604800,
//     issuer: Some("chat_server"),
//     audience: Some("chat_web"),
//     subject: None,
//     jwt_id: None,
// }

// Then You Extract the Custom Part
// let claims = self.0.verify_token::<User>(token, Some(opts))?;
// //  ↑ This is Claims<User>

// Ok(claims.custom)
// // ↑ Extract just the User from the Claims
// // Returns: User { id: 1, fullname: "Tyr", email: "..." }

// HashSet is NOT a key-value map (that's HashMap):
// Internally, HashSet uses a hash function to store values efficiently:
// Visualized:
// hash("chat_server") = 42 (hash value)
// Stores at index 42: "chat_server"
//
// But you don't see the 42 - it's internal implementation details
// So HashSet { "chat_server" } is just displaying the value, not the internal hash index.

// Some() wraps a value, and you need to unwrap it later:
// let opts = VerificationOptions {
//     allowed_issuers: Some(HashSet::from_strings(&[JWT_ISS])),
//     //                 ↑ Wraps in Option<T>
// };
// // Inside VerificationOptions, the field is:
// pub allowed_issuers: Option<HashSet<String>>;
// //                    ↑ Type is Option, not HashSet directly
// // To extract the value, you'd need to unwrap:
// if let Some(issuers) = opts.allowed_issuers {
//     // issuers is now HashSet<String> (unwrapped)
// }

// What Does .. Mean in ..Default::default()?
// The .. is struct update syntax that means "fill the rest with default values":
// Equivalent to:
// let opts = VerificationOptions {
//     allowed_issuers: Some(HashSet::from_strings(&[JWT_ISS])),
//     allowed_audiences: Some(HashSet::from_strings(&[JWT_AUD])),
//     max_future: None,
//     max_age: None,
//     allowed_subjects: None,
//     jwt_id: None,
//     // ... all other fields set to their Default
// };

// Full Verification Flow
// JWT Token String
// "eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjE...Ky5xk3zA4.X8jK9..."
//     ↓
// Split into 3 parts
//     ↓
// Decode Header, Payload, Signature from Base64URL
//     ↓
// Verify Signature using Ed25519 public key ✓
//     ↓
// Check issuer == "chat_server" ✓
//     ↓
// Check audience == "chat_web" ✓
//     ↓
// Check expiration (now < expires_at) ✓
//     ↓
// Deserialize payload JSON to Claims<User>
//     ↓
// Extract claims.custom (the User)
//     ↓
// Return User { id: 1, fullname: "Tyr Chen", email: "..." }

// Error Cases
// If ANY of these fail, ? operator returns AppError::JwtError:

// ✗ Invalid Base64URL encoding
// ✗ Signature verification fails (tampered token)
// ✗ Issuer doesn't match "chat_server"
// ✗ Audience doesn't match "chat_web"
// ✗ Token has expired (now > expires_at)
// ✗ Malformed JSON in payload

// Summary Table
// Component	Purpose
// VerificationOptions	Defines validation rules (issuer, audience, etc.)
// HashSet	Fast unique collection for allowed values
// verify_token::<User>	Verify + deserialize token as Claims<User>
// Token split	3 Base64URL-encoded parts: Header.Payload.Signature

// How JWT is Split Into 3 Parts
// The JWT string format:
// eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20iOnsiaWQiOjE...Ky5xk3zA4.X8jK9...
// ├─ Part 1 (Header)     ├─ Part 2 (Payload)              ├─ Part 3 (Signature)

// The splitting process:
// // Step 1: Find the dots
// let token = "eyJhbGciOiJFZERTQSJ9.eyJjdXN0b20i...Ky5xk3zA4.X8jK9...";
// let parts: Vec<&str> = token.split('.').collect();
// //                             ↑
// //                      Split by dot separator

// // Result:
// // parts[0] = "eyJhbGciOiJFZERTQSJ9"      (Header)
// // parts[1] = "eyJjdXN0b20i...Ky5xk3zA4"  (Payload)
// // parts[2] = "X8jK9..."                   (Signature)

// // Step 2: Decode each part from Base64URL
// let header = base64url_decode(parts[0])?;    // Binary → JSON
// let payload = base64url_decode(parts[1])?;   // Binary → JSON
// let signature = base64url_decode(parts[2])?; // Binary bytes

// // Step 3: Parse header JSON
// let header_obj = serde_json::from_slice(&header)?;
// // {
// //   "alg": "EdDSA",
// //   "typ": "JWT"
// // }

// // Step 4: Verify signature using Ed25519 public key
// let is_valid = verify_signature(
//     &header,      // Original encoded header
//     &payload,     // Original encoded payload
//     &signature,   // 64-byte Ed25519 signature
//     &public_key   // Ed25519PublicKey
// )?;

// // Step 5: Parse payload JSON to Claims<User>
// let claims_json = serde_json::from_slice(&payload)?;
// let claims: Claims<User> = serde_json::from_value(claims_json)?;
// // Claims {
// //   custom: User { id: 1, fullname: "Tyr Chen", ... },
// //   issued_at: 1685000000,
// //   expires_at: 1685604800,
// //   issuer: Some("chat_server"),
// //   audience: Some("chat_web"),
// // }

// Complete Verification Flow Code
// pub fn verify(&self, token: &str) -> Result<User, AppError> {
//     // 1. Split by dots
//     let parts: Vec<&str> = token.split('.').collect();
//     if parts.len() != 3 {
//         return Err(AppError::JwtError(...));
//     }

//     // 2. Decode and verify signature with public key
//     let claims: Claims<User> = self.0.verify_token::<User>(token, Some(opts))?;
//     //                                                       ↑              ↑
//     //                                                  Token string   Validation options

//     // 3. Check issuer and audience match
//     if claims.issuer != Some("chat_server".to_string()) {
//         return Err(AppError::JwtError(...));
//     }
//     if claims.audience != Some("chat_web".to_string()) {
//         return Err(AppError::JwtError(...));
//     }

//     // 4. Check not expired
//     if Utc::now().timestamp() > claims.expires_at {
//         return Err(AppError::JwtError(...));
//     }

//     // 5. Extract and return User
//     Ok(claims.custom)
// }

// NO dots (.) in standard Base64.

// JWT Uses Dots as Separators
// JWT wraps 3 Base64URL-encoded parts with dots:

// JWT uses Base64URL (URL-safe variant) which replaces:
// + becomes -
// / becomes _
// = padding is omitted

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn jwt_sign_verify_should_work() -> Result<()> {
        let encoding_pem = include_str!("../../fixtures/encoding.pem");
        let decoding_pem = include_str!("../../fixtures/decoding.pem");
        let ek = EncodingKey::load(encoding_pem)?;
        let dk = DecodingKey::load(decoding_pem)?;

        let user = User::new(1, "Tyr Chen", "tchen@acme.org");

        let token = ek.sign(user.clone())?;
        let user2 = dk.verify(&token)?;

        assert_eq!(user, user2);
        Ok(())
    }
}

// include_str！宏会将指定路径的文件内容作为字符串字面量包含在编译后的二进制中
// include_str! is a Rust compile-time macro that reads a file and embeds its contents as a &'static str string literal directly into your binary.

// At compile time:
// Rust looks for the file at path ../../fixtures/encoding.pem (relative to the current file)
// Reads the entire file content
// Embeds it as a string literal in your compiled binary
// encoding_pem becomes a &'static str pointing to that embedded data

// When you call Ed25519KeyPair::from_pem(pem_string)
// 1. Parse PEM wrapper
//    ↓
//    Extract: "MC4CAQAwBQYDK2VwBCIEIGb7G2hs5DE+1AL2DQpKxIBwHn6mYkEhsuntDNjg8L4w"

// 2. Base64 decode
//    ↓
//    Convert to binary bytes: [48, 46, 2, 1, 0, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, ...]

// 3. Parse ASN.1 DER structure
//    ↓
//    Extract the OID (Object Identifier) "1.3.101.112" = Ed25519
//    Extract the actual 32-byte private key seed

// 4. Derive public key mathematically
//    ↓
//    Apply Ed25519 algorithm to 32-byte seed
//    Generate 32-byte public key

// 5. Create Ed25519KeyPair
//    ↓
//    Return: Ed25519KeyPair { private_key, public_key }
