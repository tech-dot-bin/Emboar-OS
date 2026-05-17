use criterion::{black_box, criterion_group, criterion_main, Criterion};
use emboar_libs::crypto::Crypto;

fn bench_sha512(c: &mut Criterion) {
    c.bench_function("sha512_1KB", |b| {
        b.iter(|| {
            let data = black_box(vec![0u8; 1024]);
            Crypto::sha512(&data)
        });
    });

    c.bench_function("sha512_1MB", |b| {
        b.iter(|| {
            let data = black_box(vec![0u8; 1024 * 1024]);
            Crypto::sha512(&data)
        });
    });
}

fn bench_argon2(c: &mut Criterion) {
    c.bench_function("argon2id_key_derivation", |b| {
        b.iter(|| {
            let password = black_box(b"secure_password_123");
            let salt = black_box(&[0u8; 16]);
            Crypto::derive_key_argon2id(password, salt, 32)
        });
    });
}

fn bench_aes_gcm(c: &mut Criterion) {
    let key = black_box([42u8; 32]);
    let plaintext = black_box(vec![0u8; 4096]);

    c.bench_function("aes256_gcm_encrypt_4KB", |b| {
        b.iter(|| Crypto::aes256_gcm_encrypt(&key, &plaintext))
    });
}

criterion_group!(benches, bench_sha512, bench_argon2, bench_aes_gcm);
criterion_main!(benches);