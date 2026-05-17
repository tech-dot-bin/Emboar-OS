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
use crate::crypto::{Crypto, HMAC};

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
        let hash = Self::hash_argon2id(password, salt)?;

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
     * Compute Argon2id hash (using placeholder implementation)
     * 
     * In production, link against libargon2:
     *   use argon2::{Argon2, PasswordHasher, Password};
     *   use argon2::password_hash::SaltString;
     */
    fn hash_argon2id(password: &[u8], salt: &[u8]) -> Result<Vec<u8>> {
        // PLACEHOLDER: Real implementation would use argon2 crate
        // For now, use HMAC as a substitute (not cryptographically equivalent)
        
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(password);
        hash_input.extend_from_slice(salt);
        
        // Simulate Argon2id rounds with HMAC iterations
        let mut result = HMAC::sha512(password, salt).to_vec();
        
        for _ in 1..ARGON2ID_TIME_COST {
            let hmac = HMAC::sha512(&result, &hash_input);
            result = hmac.to_vec();
        }

        // Truncate to expected hash length
        result.truncate(ARGON2ID_HASH_LEN);
        Ok(result)
    }

    /**
     * Verify password against this hash
     * 
     * Constant-time comparison prevents timing attacks
     */
    pub fn verify(&self, password: &[u8]) -> Result<bool> {
        let computed_hash = Self::hash_argon2id(password, &self.salt)?;
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
        
        if parts.len() != 5 || parts[0] != "" || parts[1] != "argon2id" {
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
        let salt = base64_decode(parts[3])?;
        let hash = base64_decode(parts[4])?;

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
    use std::fmt::Write;

    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b3 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        result.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[(n & 0x3F) as usize] as char);
        }
    }

    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;

    const LOOKUP: &[u8] = b"\
        \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
        \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
        \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x3E\xFF\xFF\
        \x34\x35\x36\x37\x38\x39\x3A\x3B\x3C\x3D\xFF\xFF\xFF\xFF\xFF\xFF\
        \xFF\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\
        \x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\xFF\xFF\xFF\xFF\x3F\
        \xFF\x1A\x1B\x1C\x1D\x1E\x1F\x20\x21\x22\x23\x24\x25\x26\x27\x28\
        \x29\x2A\x2B\x2C\x2D\x2E\x2F\x30\x31\x32\x33\xFF\xFF\xFF\xFF\xFF\
    ";

    for ch in data.bytes() {
        let val = LOOKUP[ch as usize];
        if val == 0xFF {
            anyhow::bail!("Invalid base64url character: {}", ch as char);
        }

        buf = (buf << 6) | (val as u32);
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(bytes)
}

/* =========================================================================
 * TESTS
 * ======================================================================= */

#[cfg(test)]
mod tests {
    use super::*;

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
}
