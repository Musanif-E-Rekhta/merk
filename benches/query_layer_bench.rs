use criterion::{criterion_group, criterion_main, Criterion};
use merk::services::auth::hash_password;

fn bench_password_hash(c: &mut Criterion) {
    c.bench_function("argon2_hash", |b| {
        b.iter(|| hash_password("supersecretpassword123!"))
    });
}

criterion_group!(benches, bench_password_hash);
criterion_main!(benches);
