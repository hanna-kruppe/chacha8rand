use std::hint::black_box;

use chacha8rand::{Backend, ChaCha8, Seed};
use divan::{counter::BytesCount, Bencher};

fn main() {
    // Run registered benchmarks.
    divan::main();
}

fn bench(bencher: Bencher, kilobytes: u32, backend: Backend) {
    const SEED: &[u8; 32] = b"thisisjustabenchthisisjustabench";
    let words = kilobytes * 1024 / 4;
    bencher.counter(BytesCount::new(words * 4)).bench(|| {
        let mut rng = ChaCha8::with_backend(Seed::from(SEED), backend);
        for _ in 0..words {
            black_box(rng.next_u32());
        }
    })
}

const LENS: &[u32] = &[1, 64, 512];

#[divan::bench(args = LENS)]
fn scalar(bencher: Bencher, kilobytes: u32) {
    bench(bencher, kilobytes, Backend::scalar());
}

#[divan::bench(args = LENS)]
fn simd128(bencher: Bencher, kilobytes: u32) {
    bench(bencher, kilobytes, Backend::simd128());
}
