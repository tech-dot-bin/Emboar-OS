#![no_main]
use libfuzzer_sys::fuzz_target;
use emboar_crypto::Crypto;

fuzz_target!(|data: &[u8]| {
    // Split data: first 16 bytes = salt, rest = password
    if data.len() < 16 {
        return;
    }

    let salt = &data[..16];
    let password = &data[16..];

    // Should never panic or crash
    if let Ok(hash) = Crypto::derive_key_argon2id(password, salt, 32) {
        assert_eq!(hash.len(), 32);
    }
});