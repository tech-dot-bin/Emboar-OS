/*
 * Password hashing utilities
 * 
 * Implements OWASP-recommended password hashing with Argon2id:
 *   - Time cost: 2 iterations
 *   - Memory cost: 19 MiB (19456 KiB)
 *   - Parallelism: 1 thread
 *   - Salt: 16 bytes (128 bits)
 *   - Hash length: 32 bytes (256 bits)
 * 
 * Reference: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
 */

use anyhow::{Context, Result};
use crate::crypto::Crypto;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};


/* =========================================================================
 * ARGON2ID PARAMETERS
 * ======================================================================= */

/// OWASP recommended parameters for Argon2id
pub const ARGON2ID_TIME_COST: u32 = 2;           // iterations
pub const ARGON2ID_MEMORY_COST: u32 = 19456;     // KiB (19 MiB)
pub const ARGON2ID_PARALLELISM: u32 = 1;         // threads
pub const ARGON2ID_SALT_LEN: usize = 16;         // bytes
pub const ARGON2ID_HASH_LEN: usize = 32;         // bytes

/* =========================================================================
 * PASSWORD HASH STRUCTURE
 * ======================================================================= */

/**
 * Password hash with metadata
 * 
 * Format for persistent storage: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
 * 
 * Components:
 *   - algorithm: "argon2id"
 *   - version: 19 (Argon2id version)
 *   - m: memory cost in KiB
 *   - t: time cost (iterations)
 *   - p: parallelism (threads)
 *   - salt: base64url-encoded salt (16 bytes = 22 chars)
 *   - hash: base64url-encoded hash (32 bytes = 43 chars)
 */
pub struct PasswordHash {
    pub algorithm: String,
    pub version: u8,
    pub time_cost: u32,
    pub memory_cost: u32,
    pub parallelism: u32,
    pub salt: Vec<u8>,
    pub hash: Vec<u8>,
}

impl PasswordHash {
    /**
     * Create new password hash by hashing a password
     * 
     * Automatically generates random salt and computes hash
     */
    pub fn new(password: &[u8]) -> Result<Self> {
        let salt = Crypto::random_bytes(ARGON2ID_SALT_LEN)?;
        Self::with_salt(password, &salt)
    }

    /**
     * Create password hash with specific salt
     * 
     * Used for testing or when salt is known
     */
    pub fn with_salt(password: &[u8], salt: &[u8]) -> Result<Self> {
        if salt.len() != ARGON2ID_SALT_LEN {
            anyhow::bail!(
                "Invalid salt length: {} (expected {})",
                salt.len(),
                ARGON2ID_SALT_LEN
            );
        }

        // Hash password using Argon2id
        let hash = Self::hash_argon2id(
            password,
            salt,
            ARGON2ID_MEMORY_COST,
            ARGON2ID_TIME_COST,
            ARGON2ID_PARALLELISM,
        )?;

        Ok(PasswordHash {
            algorithm: "argon2id".to_string(),
            version: 19,
            time_cost: ARGON2ID_TIME_COST,
            memory_cost: ARGON2ID_MEMORY_COST,
            parallelism: ARGON2ID_PARALLELISM,
            salt: salt.to_vec(),
            hash,
        })
    }

    /**
     * Compute Argon2id hash using specific parameters
     */
    fn hash_argon2id(
        password: &[u8],
        salt: &[u8],
        m: u32,
        t: u32,
        p: u32,
    ) -> Result<Vec<u8>> {
        // Call the actual implementation in crypto.rs
        Crypto::derive_key_argon2id(password, salt, ARGON2ID_HASH_LEN, m, t, p)
    }

    /**
     * Verify password against this hash
     * 
     * Constant-time comparison prevents timing attacks
     */
    pub fn verify(&self, password: &[u8]) -> Result<bool> {
        let computed_hash = Self::hash_argon2id(
            password,
            &self.salt,
            self.memory_cost,
            self.time_cost,
            self.parallelism,
        )?;
        Ok(Crypto::constant_time_compare(&computed_hash, &self.hash))
    }

    /**
     * Parse PHC string format
     * 
     * Format: $argon2id$v=19$m=19456,t=2,p=1$<salt_b64>$<hash_b64>
     * 
     * Example:
     * $argon2id$v=19$m=19456,t=2,p=1$RZv3vwtj+REb+HN7cVrVOw$n45SvqKPjEvC31ltPkWNLWec0SYJQ4yWFd84preKT7E
     */
    pub fn from_phc_string(phc: &str) -> Result<Self> {
        let parts: Vec<&str> = phc.split('$').collect();
        
        if parts.len() != 6 || parts[0] != "" || parts[1] != "argon2id" {
            anyhow::bail!("Invalid PHC format");
        }

        // Parse version: v=19
        let version = parts[2]
            .strip_prefix("v=")
            .context("Missing version")?
            .parse::<u8>()?;

        // Parse parameters: m=19456,t=2,p=1
        let params_str = parts[3];
        let params: std::collections::HashMap<&str, u32> = params_str
            .split(',')
            .filter_map(|param| {
                let mut kv = param.split('=');
                match (kv.next(), kv.next()) {
                    (Some(k), Some(v)) => v.parse().ok().map(|v| (k, v)),
                    _ => None,
                }
            })
            .collect();

        let memory_cost = *params.get("m").context("Missing memory cost")?;
        let time_cost = *params.get("t").context("Missing time cost")?;
        let parallelism = *params.get("p").context("Missing parallelism")?;

        // Decode salt and hash from base64url
        let salt = base64_decode(parts[4])?;
        let hash = base64_decode(parts[5])?;

        Ok(PasswordHash {
            algorithm: "argon2id".to_string(),
            version,
            time_cost,
            memory_cost,
            parallelism,
            salt,
            hash,
        })
    }

    /**
     * Convert to PHC string format for storage
     */
    pub fn to_phc_string(&self) -> String {
        let salt_b64 = base64_encode(&self.salt);
        let hash_b64 = base64_encode(&self.hash);

        format!(
            "$argon2id$v={}$m={},t={},p={}${}${}",
            self.version,
            self.memory_cost,
            self.time_cost,
            self.parallelism,
            salt_b64,
            hash_b64
        )
    }
}

/* =========================================================================
 * BASE64URL ENCODING/DECODING
 * ======================================================================= */

fn base64_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(data).map_err(|e| anyhow::anyhow!("Base64 decode error: {}", e))
}

/* =========================================================================
 * TESTS
 * ======================================================================= */

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{password_hash::{SaltString, PasswordHasher}, Argon2};

    #[test]
    fn test_password_hash_create() {
        let password = b"my_secure_password";
        let hash = PasswordHash::new(password).unwrap();
        
        assert_eq!(hash.algorithm, "argon2id");
        assert_eq!(hash.salt.len(), ARGON2ID_SALT_LEN);
        assert_eq!(hash.hash.len(), ARGON2ID_HASH_LEN);
    }

    #[test]
    fn test_password_verify() {
        let password = b"my_password";
        let hash = PasswordHash::new(password).unwrap();
        
        assert!(hash.verify(password).unwrap());
        assert!(!hash.verify(b"wrong_password").unwrap());
    }

    #[test]
    fn test_phc_format() {
        let password = b"test";
        let hash = PasswordHash::new(password).unwrap();
        
        let phc = hash.to_phc_string();
        assert!(phc.starts_with("$argon2id$"));
        
        // Parse it back
        let parsed = PasswordHash::from_phc_string(&phc).unwrap();
        assert_eq!(parsed.algorithm, "argon2id");
    }

    #[test]
    fn test_password_hashing() {
        let password = "secure_password";
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        let hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        assert!(hash.to_string().contains("argon2id"));
    }

    #[test]
    fn test_invalid_password_hashing() {
        let password = ""; // Empty password
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        let result = argon2.hash_password(password.as_bytes(), &salt);
        assert!(result.is_ok());
    }
}
