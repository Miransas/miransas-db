# Benchmarks

Two kinds of benchmarks live here.

---

## Criterion micro-benchmarks (Rust)

Located in `backend/benches/crypto_bench.rs`.  
These benchmark AES-256-GCM encryption/decryption and JWT signing/verification
without any network or DB I/O.

```bash
cd backend
cargo bench
# HTML report → backend/target/criterion/
```

---

## k6 HTTP load tests

Located in `benchmarks/k6/`.  
Requires [k6](https://k6.io/docs/get-started/installation/) to be installed.

### Smoke test (1 VU, 1 iteration — sanity check)

```bash
k6 run benchmarks/k6/smoke.js \
  -e BASE_URL=http://localhost:3001 \
  -e ADMIN_PASSWORD=your-admin-password
```

### Load test (ramp 0→50 VUs over ~4 minutes)

```bash
k6 run benchmarks/k6/load.js \
  -e BASE_URL=http://localhost:3001 \
  -e ADMIN_PASSWORD=your-admin-password
```

### With HTML report

```bash
k6 run --out json=results.json benchmarks/k6/load.js
```

---

## Thresholds

| Metric | Target |
|---|---|
| Error rate | < 1 % |
| p95 latency | < 800 ms |
| p99 latency | < 2 000 ms |
