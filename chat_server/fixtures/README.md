
openssl genpkey -algorithm ed25519 -out encoding.pem
openssl pkey -in encoding.pem -pubout -out decoding.pem



cargo nextest run -- sign_up_should_work
cargo nextest run

touch test.rest
cargo run --bin chat-server


cargo run --release



# What Does PEM Mean?

**PEM** stands for **Privacy-Enhanced Mail**, though it's now a universal standard for encoding cryptographic data as text.

## Structure

PEM is a text format with three parts:

```
-----BEGIN PRIVATE KEY-----
Base64EncodedData...
-----END PRIVATE KEY-----
```

- **Header**: `-----BEGIN [TYPE] KEY-----`
- **Body**: Base64-encoded binary key data
- **Footer**: `-----END [TYPE] KEY-----`

## In Your Commands

```bash
# Generate Ed25519 private key, save as PEM text file
openssl genpkey -algorithm ed25519 -out encoding.pem

# Extract public key from private key, save as PEM text file
openssl pkey -in encoding.pem -pubout -out decoding.pem
```

Both files are **plain text** containing Base64-encoded Ed25519 key material.

## Why PEM?

1. **Text-based**: Easy to store in files, config files (like your `app.yml`), or version control
2. **Portable**: Works across different systems and programming languages
3. **Self-describing**: The header/footer tell you what type of key it is
4. **Standard**: Widely supported by cryptographic libraries (like `jwt_simple` in your code)

## In Your Code

```rust
// app.yml stores PEM as a multi-line string
sk: |
  -----BEGIN PRIVATE KEY-----
  MC4CAQAwBQYDK2VwBCIEIGb7G2hs5DE+1AL2DQpKxIBwHn6mYkEhsuntDNjg8L4w
  -----END PRIVATE KEY-----

// Then loaded and used for JWT signing
let ek = EncodingKey::load(&config.auth.sk)?;
let token = ek.sign(user)?;
```

**Short answer**: PEM is a standard text format for storing keys. It's readable, portable, and supported everywhere.






# Ed25519 Explanation

**Ed25519** is a modern public-key signature algorithm based on elliptic curve cryptography.

## Key Characteristics

- **Type**: Asymmetric cryptography (uses a key pair: private + public key)
- **Curve**: Edwards Elliptic Curve 25519
- **Security**: 128-bit security level (equivalent to 3072-bit RSA)
- **Speed**: Very fast compared to RSA
- **Key Size**: 
  - Private key: 32 bytes
  - Public key: 32 bytes
  - Signature: 64 bytes

## How It Works in Your App

```
Private Key (sk)          Public Key (pk)
    ↓                          ↓
EncodingKey            DecodingKey
    ↓                          ↓
sign(user)  ------→   ✓ Verify Token
   JWT created         Extract User
```

1. **Server** uses the private key to **sign** JWT tokens (create signatures)
2. **Client** sends token in requests
3. **Server** uses the public key to **verify** the token is authentic and hasn't been tampered with

## Why Ed25519?

- **Modern standard** - Recommended by security experts (NIST, NSA)
- **Simpler** - No parameter selection headaches like RSA
- **Faster** - Better performance than RSA for same security level
- **Deterministic** - Same input always produces same signature (better for testing)
- **No weak keys** - Unlike RSA, all keys are suitable for use

Your `jwt.rs` implements this via the `jwt_simple` crate with Ed25519 key pairs for secure token authentication.