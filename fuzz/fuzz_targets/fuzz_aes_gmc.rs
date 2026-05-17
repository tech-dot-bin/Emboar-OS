#![no_main]
use libfuzzer_sys::fuzz_target;
use emboar_crypto::Crypto;

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let key = &data[..32];
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(key);

    let plaintext = &data[32..];

    // Encryption should not panic
    if let Ok(encrypted) = Crypto::aes256_gcm_encrypt(&key_array, plaintext) {
        // Decryption should succeed with same key
        if let Ok(decrypted) = Crypto::aes256_gcm_decrypt(&key_array, &encrypted) {
            assert_eq!(plaintext, &decrypted[..]);
        }
    }
});