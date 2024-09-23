use std::hint::black_box;

use chacha8rand::{guts, ChaCha8, RefillFn, Seed};
use divan::{counter::BytesCount, Bencher};

fn main() {
    // Run registered benchmarks.
    divan::main();
}

fn bench(bencher: Bencher, kilobytes: u32, refill: RefillFn) {
    const SEED: &[u8; 32] = b"thisisjustabenchthisisjustabench";
    let words = kilobytes * 1024 / 4;
    bencher.counter(BytesCount::new(words * 4)).bench(|| {
        let mut rng = ChaCha8::new_with_impl(Seed::from(SEED), refill);
        for _ in 0..words {
            black_box(rng.next_u32());
        }
    })
}

const LENS: &[u32] = &[1, 64, 512];

#[divan::bench(args = LENS)]
fn scalar(bencher: Bencher, kilobytes: u32) {
    bench(bencher, kilobytes, guts::scalar::fill_buf);
}

#[divan::bench(args = LENS)]
fn simd128(bencher: Bencher, kilobytes: u32) {
    bench(bencher, kilobytes, guts::simd128::fill_buf);
}
