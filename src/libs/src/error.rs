/*
 * Error types for Emboar OS
 */

use std::fmt;

/// Emboar OS custom error type
#[derive(Debug)]
pub enum EmboarError {
    CryptoError(String),
    PasswordError(String),
    EncryptionError(String),
    DecryptionError(String),
    FormatError(String),
    IOError(String),
}

impl fmt::Display for EmboarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmboarError::CryptoError(msg) => write!(f, "Cryptographic error: {}", msg),
            EmboarError::PasswordError(msg) => write!(f, "Password error: {}", msg),
            EmboarError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            EmboarError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            EmboarError::FormatError(msg) => write!(f, "Format error: {}", msg),
            EmboarError::IOError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for EmboarError {}

impl From<ring::error::Unspecified> for EmboarError {
    fn from(err: ring::error::Unspecified) -> Self {
        EmboarError::CryptoError(format!("Ring crypto error: {:?}", err))
    }
}

/// Result type for Emboar OS operations
pub type Result<T> = std::result::Result<T, EmboarError>;
