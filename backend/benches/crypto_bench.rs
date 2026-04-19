use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use miransas_db::utils::{crypto, jwt};

// ── AES-256-GCM ───────────────────────────────────────────────────────────────

fn bench_crypto(c: &mut Criterion) {
    let key = "bench-secret-key-exactly-32chars!";

    let mut group = c.benchmark_group("crypto");

    for payload in ["short", "a somewhat longer secret value for testing purposes"] {
        group.bench_with_input(
            BenchmarkId::new("encrypt", payload.len()),
            payload,
            |b, p| b.iter(|| crypto::encrypt(key, p).unwrap()),
        );

        let encrypted = crypto::encrypt(key, payload).unwrap();
        group.bench_with_input(
            BenchmarkId::new("decrypt", payload.len()),
            &encrypted,
            |b, enc| b.iter(|| crypto::decrypt(key, enc).unwrap()),
        );
    }

    group.finish();
}

// ── JWT HS256 ────────────────────────────────────────────────────────────────

fn bench_jwt(c: &mut Criterion) {
    let secret = "bench-jwt-secret-key-exactly-32-chars!!";

    let mut group = c.benchmark_group("jwt");

    group.bench_function("create_token", |b| {
        b.iter(|| jwt::create_token(secret).unwrap())
    });

    let token = jwt::create_token(secret).unwrap();
    group.bench_function("verify_token", |b| {
        b.iter(|| jwt::verify_token(secret, &token).unwrap())
    });

    group.finish();
}

criterion_group!(benches, bench_crypto, bench_jwt);
criterion_main!(benches);
