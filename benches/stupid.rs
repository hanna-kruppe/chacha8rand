use std::{array, hint::black_box};

use arrayref::array_ref;
use chacha8rand::{Backend, ChaCha8, Seed};
use divan::{counter::BytesCount, Bencher};

fn main() {
    divan::main();
}

const SEED: &[u8; 32] = b"thisisjustabenchthisisjustabench";

#[inline(never)]
fn bench_next_u32(bencher: Bencher, backend: Backend) {
    let backend = black_box(backend);
    let mut rng = ChaCha8::with_backend(Seed::from(SEED), backend);
    bencher.bench_local(|| {
        black_box(rng.next_u32());
    });
}

#[inline(never)]
fn bench_bulk(bencher: Bencher, backend: Backend) {
    let mut key = array::from_fn(|i| u32::from_le_bytes(*array_ref![SEED, i * 4, 4]));
    let mut buf = [0; 256];
    // Arguably, the 32 bytes that are used as next key should not be counted because they're not
    // useful output. However, the Go version has a benchmark that's equivalent to this one (can be
    // run with `go test -bench=Block internal/chacha8rand`) which counts the full block, and it's
    // nice to be able to compare the report throughput directly.
    let counter = BytesCount::u32(256);
    bencher.counter(counter).bench_local(|| {
        black_box(&mut key);
        backend.refill(&key, &mut buf);
        key = *array_ref![buf, 256 - 8, 8];
        black_box(&buf);
    });
}

#[divan::bench]
fn next_u32_scalar(bencher: Bencher) {
    bench_next_u32(bencher, Backend::scalar());
}

#[divan::bench]
fn next_u32_simd128(bencher: Bencher) {
    bench_next_u32(bencher, Backend::simd128());
}

#[divan::bench]
#[cfg(target_arch = "x86_64")]
fn next_u32_avx2(bencher: Bencher) {
    bench_next_u32(bencher, Backend::avx2().expect("avx2 is required for this"));
}

#[divan::bench]
fn next_u32_nop(bencher: Bencher) {
    bench_next_u32(
        bencher,
        Backend::totally_wrong_stub_for_testing_that_breaks_everything_if_you_actually_use_it(),
    );
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
