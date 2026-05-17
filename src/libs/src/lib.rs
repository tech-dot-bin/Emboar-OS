/*
 * Emboar OS Shared Libraries
 * 
 * Provides common utilities for:
 *   - Cryptographic operations (Argon2id, AES-256, SHA-512)
 *   - Password hashing
 *   - Encryption/decryption
 *   - Data serialization
 * 
 * File: src/libs/src/lib.rs
 */

pub mod crypto;
pub mod password;
pub mod error;

pub use crypto::{Crypto, EncryptionAlgorithm};
pub use password::PasswordHash;
pub use error::{EmboarError, Result};

/* =========================================================================
 * VERSION AND METADATA
 * ======================================================================= */

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/* =========================================================================
 * PRELUDE - Re-export common items
 * ======================================================================= */

pub mod prelude {
    pub use crate::{
        crypto::{Crypto, EncryptionAlgorithm},
        password::PasswordHash,
        error::{EmboarError, Result},
    };
}