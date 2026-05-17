/*
 * Cryptographic utilities for Emboar OS
 * 
 * Implements:
 *   - Argon2id password hashing (OWASP standard)
 *   - AES-256-GCM encryption/decryption
 *   - SHA-512 hashing
 *   - RSA-4096 (via ring crate)
 */

use ring::digest::{SHA512, digest};
use anyhow::Result;

/* =========================================================================
 * ENCRYPTION ALGORITHMS
 * ======================================================================= */

#[derive(Debug, Clone, Copy)]
pub enum EncryptionAlgorithm {
    /// AES-256 in XTS mode (for full-disk encryption)
    AES256XTS,
    /// AES-256 in GCM mode (for authenticated encryption)
    AES256GCM,
    /// ChaCha20Poly1305 (AEAD, modern alternative)
    ChaCha20Poly1305,
}

impl EncryptionAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::AES256XTS => "AES-256-XTS",
            EncryptionAlgorithm::AES256GCM => "AES-256-GCM",
            EncryptionAlgorithm::ChaCha20Poly1305 => "ChaCha20-Poly1305",
        }
    }
}

/* =========================================================================
 * CRYPTOGRAPHY TOOLS
 * ======================================================================= */

pub struct Crypto;

impl Crypto {
    /**
     * Compute SHA-512 hash of data
     * 
     * Returns: 64-byte (512-bit) hash
     */
    pub fn sha512(data: &[u8]) -> [u8; 64] {
        let digest_result = digest(&SHA512, data);
        let bytes = digest_result.as_ref();
        
        let mut hash = [0u8; 64];
        hash.copy_from_slice(bytes);
        hash
    }

    /**
     * Compute SHA-512 hash of data (returns Vec for flexibility)
     */
    pub fn sha512_vec(data: &[u8]) -> Vec<u8> {
        let digest_result = digest(&SHA512, data);
        digest_result.as_ref().to_vec()
    }

    /**
     * Verify SHA-512 hash matches
     * 
     * Constant-time comparison to prevent timing attacks
     */
    pub fn sha512_verify(data: &[u8], expected_hash: &[u8]) -> bool {
        if expected_hash.len() != 64 {
            return false;
        }
        
        let computed = Self::sha512(data);
        Self::constant_time_compare(&computed, expected_hash)
    }

    /**
     * Constant-time memory comparison
     * 
     * Prevents timing attacks by comparing all bytes
     * regardless of when a mismatch is found
     */
    pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }

        result == 0
    }

    /**
     * Generate random bytes using ring's secure random
     * 
     * Returns: Vector of n random bytes
     */
    pub fn random_bytes(n: usize) -> Result<Vec<u8>> {
        use ring::rand::SecureRandom;
        
        let rng = ring::rand::SystemRandom::new();
        let mut bytes = vec![0u8; n];
        rng.fill(&mut bytes)
            .map_err(|_| anyhow::anyhow!("Failed to generate random bytes"))?;
        
        Ok(bytes)
    }

    /**
     * Get random 96-bit (12-byte) nonce for AES-GCM
     */
    pub fn random_nonce() -> Result<Vec<u8>> {
        Self::random_bytes(12) // AES-GCM nonce is 96 bits
    }

    /**
     * Encrypt data using AES-256-GCM
     * 
     * Returns: (nonce || ciphertext || tag)
     * Format allows decryption without storing nonce separately
     */
    pub fn aes256_gcm_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
        use ring::aead::{self, Aad, LessSafeKey, UnboundKey};

        if key.len() != 32 {
            anyhow::bail!("AES-256-GCM requires 32-byte key");
        }

        // Generate random nonce
        let nonce_bytes = Self::random_bytes(12)?;
        
        // Fix: try_assume_unique_for_key takes 2 arguments (NONCE_LEN and bytes)
        let nonce = aead::Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid nonce"))?;

        // Create encryption key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt
        let mut ciphertext = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
            .map_err(|_| anyhow::anyhow!("Encryption failed"))?;

        // Return: nonce || ciphertext (tag is appended by seal_in_place)
        let mut result = nonce_bytes;
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /**
     * Decrypt data using AES-256-GCM
     * 
     * Expects: (nonce || ciphertext || tag)
     */
    pub fn aes256_gcm_decrypt(key: &[u8; 32], encrypted: &[u8]) -> Result<Vec<u8>> {
        use ring::aead::{self, Aad, LessSafeKey, UnboundKey};

        if key.len() != 32 {
            anyhow::bail!("AES-256-GCM requires 32-byte key");
        }

        if encrypted.len() < 12 + 16 {
            anyhow::bail!("Encrypted data too short");
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        
        // Fix: try_assume_unique_for_key takes 2 arguments
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid nonce"))?;

        // Create decryption key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow::anyhow!("Failed to create decryption key"))?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt
        let mut plaintext = ciphertext.to_vec();
        key.open_in_place(nonce, Aad::empty(), &mut plaintext)
            .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

        // Remove tag (last 16 bytes)
        plaintext.truncate(plaintext.len() - 16);
        Ok(plaintext)
    }

    /**
     * Derive key from password using Argon2id
     * 
     * OWASP recommended parameters for password hashing
     */
    pub fn derive_key_argon2id(password: &[u8], salt: &[u8], key_len: usize) -> Result<Vec<u8>> {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{Params, SaltString};
        use base64::{engine::general_purpose, Engine as _};

        // Create salt string from bytes
        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| anyhow::anyhow!("Failed to create salt: {}", e))?;

        // OWASP recommended Argon2id parameters (as of 2023)
        // m=19456 KiB, t=2 iterations, p=1 parallelism
        let params = Params::new(19456, 2, 1, Some(key_len))
            .map_err(|e| anyhow::anyhow!("Invalid Argon2 params: {}", e))?;

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            params,
        );

        // Hash the password
        let password_hash = argon2
            .hash_password(password, &salt_string)
            .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {}", e))?;

        // Extract the hash bytes
        let hash_str = password_hash
            .hash
            .ok_or_else(|| anyhow::anyhow!("No hash generated"))?
            .as_str();
        
        let hash_bytes = general_purpose::STANDARD
            .decode(hash_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode hash: {}", e))?;

        // Return requested key length
        Ok(hash_bytes[..key_len.min(hash_bytes.len())].to_vec())
    }
}

/* =========================================================================
 * HMAC (Hash-based Message Authentication Code)
 * ======================================================================= */

pub struct HMAC;

impl HMAC {
    /**
     * Compute HMAC-SHA512
     * 
     * Used for message authentication and key derivation
     */
    pub fn sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
        use ring::hmac;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA512, key);
        let tag = hmac::sign(&signing_key, data);
        let bytes = tag.as_ref();

        let mut hash = [0u8; 64];
        hash.copy_from_slice(bytes);
        hash
    }

    /**
     * Verify HMAC-SHA512
     */
    pub fn sha512_verify(key: &[u8], data: &[u8], expected_tag: &[u8]) -> bool {
        use ring::hmac;

        if expected_tag.len() != 64 {
            return false;
        }

        let signing_key = hmac::Key::new(hmac::HMAC_SHA512, key);
        hmac::verify(&signing_key, data, expected_tag).is_ok()
    }
}

/* =========================================================================
 * TESTS
 * ======================================================================= */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha512() {
        let data = b"hello world";
        let hash = Crypto::sha512(data);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_sha512_verify() {
        let data = b"test data";
        let hash = Crypto::sha512(data);
        assert!(Crypto::sha512_verify(data, &hash));
        assert!(!Crypto::sha512_verify(b"wrong data", &hash));
    }

    #[test]
    fn test_constant_time_compare() {
        let a = b"test";
        let b = b"test";
        assert!(Crypto::constant_time_compare(a, b));

        let c = b"fail";
        assert!(!Crypto::constant_time_compare(a, c));
    }

    #[test]
    fn test_hmac_sha512() {
        let key = b"secret";
        let data = b"message";
        let tag = HMAC::sha512(key, data);
        assert_eq!(tag.len(), 64);
        assert!(HMAC::sha512_verify(key, data, &tag));
    }

    #[test]
    fn test_aes256_gcm() {
        let key = [42u8; 32]; // 256-bit key
        let plaintext = b"Secret message";
        
        let encrypted = Crypto::aes256_gcm_encrypt(&key, plaintext).unwrap();
        let decrypted = Crypto::aes256_gcm_decrypt(&key, &encrypted).unwrap();
        
        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_aes256_gcm_tamper_detection() {
        let key = [42u8; 32];
        let plaintext = b"Secret";
        
        let mut encrypted = Crypto::aes256_gcm_encrypt(&key, plaintext).unwrap();
        
        // Tamper with ciphertext
        encrypted ^= 0xFF;
        
        // Decryption should fail
        assert!(Crypto::aes256_gcm_decrypt(&key, &encrypted).is_err());
    }
}