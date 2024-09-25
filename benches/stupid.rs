use std::{array, hint::black_box};

use arrayref::array_ref;
use chacha8rand::{Backend, ChaCha8, Seed};
use divan::{counter::BytesCount, Bencher};

fn main() {
    divan::main();
}

const SEED: &[u8; 32] = b"thisisjustabenchthisisjustabench";

fn bench_next(bencher: Bencher, backend: Backend) {
    let backend = black_box(backend);
    let mut rng = ChaCha8::with_backend(Seed::from(SEED), backend);
    bencher.bench_local(|| {
        black_box(rng.next_u32());
    });
}

fn bench_bulk(bencher: Bencher, backend: Backend) {
    let mut key = array::from_fn(|i| u32::from_le_bytes(*array_ref![SEED, i * 4, 4]));
    let mut buf = [0; 256];
    bencher.counter(BytesCount::u32(256 - 8)).bench_local(|| {
        backend.refill(&key, &mut buf);
        key = *array_ref![buf, 256 - 8, 8];
    });
}

#[divan::bench]
fn next_scalar(bencher: Bencher) {
    bench_next(bencher, Backend::scalar());
}

#[divan::bench]
fn next_simd128(bencher: Bencher) {
    bench_next(bencher, Backend::simd128());
}

#[divan::bench]
#[cfg(target_arch = "x86_64")]
fn next_avx2(bencher: Bencher) {
    bench_next(bencher, Backend::avx2().expect("avx2 is required for this"));
}

#[divan::bench]
fn bulk_scalar(bencher: Bencher) {
    bench_bulk(bencher, Backend::scalar());
}

#[divan::bench]
fn bulk_simd128(bencher: Bencher) {
    bench_bulk(bencher, Backend::simd128());
}

#[divan::bench]
fn bulk_avx2(bencher: Bencher) {
    bench_bulk(bencher, Backend::avx2().expect("avx2 is required for this"));
}
