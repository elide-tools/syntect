//! Benchmarks for comparing compression backends.
//!
//! Run with a specific compression backend:
//! - `cargo bench --bench compression --features compression-flate2`
//! - `cargo bench --bench compression --features compression-zstd`
//! - `cargo bench --bench compression --features compression-lz4`

use criterion::{criterion_group, criterion_main, Bencher, Criterion};

#[cfg(any(
    feature = "compression-flate2",
    feature = "compression-zstd",
    feature = "compression-lz4"
))]
fn bench_compress(b: &mut Bencher) {
    // Sample data similar to syntax definitions
    let data = br#"{"contexts":[{"meta_scope":"source.rust","patterns":[{"match":"\\b(fn|let|mut|const|static)\\b","scope":"keyword.declaration"}]}]}"#.repeat(50);

    b.iter(|| syntect::compression::compress(&data).unwrap());
}

#[cfg(any(
    feature = "compression-flate2",
    feature = "compression-zstd",
    feature = "compression-lz4"
))]
fn bench_decompress(b: &mut Bencher) {
    let data = br#"{"contexts":[{"meta_scope":"source.rust","patterns":[{"match":"\\b(fn|let|mut|const|static)\\b","scope":"keyword.declaration"}]}]}"#.repeat(50);
    let compressed = syntect::compression::compress(&data).unwrap();

    b.iter(|| syntect::compression::decompress(&compressed).unwrap());
}

#[cfg(any(
    feature = "compression-flate2",
    feature = "compression-zstd",
    feature = "compression-lz4"
))]
fn bench_load_defaults(b: &mut Bencher) {
    use syntect::parsing::SyntaxSet;
    b.iter(|| SyntaxSet::load_defaults_newlines());
}

#[cfg(any(
    feature = "compression-flate2",
    feature = "compression-zstd",
    feature = "compression-lz4"
))]
fn compression_benchmark(c: &mut Criterion) {
    c.bench_function("compress", bench_compress);
    c.bench_function("decompress", bench_decompress);
    c.bench_function("load_defaults", bench_load_defaults);
}

#[cfg(not(any(
    feature = "compression-flate2",
    feature = "compression-zstd",
    feature = "compression-lz4"
)))]
fn compression_benchmark(_c: &mut Criterion) {
    // No compression backend enabled
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = compression_benchmark
}
criterion_main!(benches);
