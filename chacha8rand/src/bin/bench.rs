use std::{
    array, cmp,
    hint::black_box,
    time::{Duration, Instant},
};

use arrayref::array_ref;
use chacha8rand::{Backend, Buffer, ChaCha8, Seed};

fn main() {
    println!("label,min,p10,p50,p90,max,min_repeats,max_repeats");
    for bench in collect_benchmarks() {
        let label = bench.label.clone();
        let NanosPerOp {
            min,
            p10,
            p50,
            p90,
            max,
            min_repeats,
            max_repeats,
        } = run_benchmark(bench);
        println!(
            "{label},{min:.2},{p10:.2},{p50:.2},{p90:.2},{max:.2},{min_repeats},{max_repeats}"
        );
        assert!(min <= p10 && p10 <= p50 && p50 <= p90 && p90 <= max);
        assert!(min_repeats <= max_repeats);
    }
}

fn collect_benchmarks() -> Vec<Benchmark> {
    let mut benchmarks = Vec::new();

    benchmarks.push(bench_next_u32("scalar", Backend::scalar()));
    if let Some(sse2) = Backend::x86_sse2() {
        benchmarks.push(bench_next_u32("sse2", sse2));
    }
    if let Some(avx2) = Backend::x86_avx2() {
        benchmarks.push(bench_next_u32("avx2", avx2));
    }
    if let Some(neon) = Backend::aarch64_neon() {
        benchmarks.push(bench_next_u32("neon", neon));
    }
    if let Some(simd128) = Backend::wasm32_simd128() {
        benchmarks.push(bench_next_u32("simd128", simd128));
    }

    benchmarks.push(bench_bulk("scalar", Backend::scalar()));
    if let Some(sse2) = Backend::x86_sse2() {
        benchmarks.push(bench_bulk("sse2", sse2));
    }
    if let Some(avx2) = Backend::x86_avx2() {
        benchmarks.push(bench_bulk("avx2", avx2));
    }
    if let Some(neon) = Backend::aarch64_neon() {
        benchmarks.push(bench_bulk("neon", neon));
    }
    if let Some(simd128) = Backend::wasm32_simd128() {
        benchmarks.push(bench_bulk("simd128", simd128));
    }

    #[cfg(feature = "rand_core_0_6")]
    benchmarks.push(Benchmark {
        label: "bulk/fill_bytes".into(),
        work: Box::new(move |n| {
            use rand_core::RngCore;
            let mut rng = ChaCha8::new(Seed::from(SEED));
            // Each iteration generates 1024 - 32 bytes of output + 32 bytes of new key material to
            // match the work done by the other "bulk" benchmarks.
            let mut output = [0; 1024 - 32];
            for _ in 0..n {
                let dest = black_box(&mut output);
                rng.fill_bytes(dest);
            }
        }),
    });

    benchmarks
}

#[test]
fn test_benchmarks() {
    for mut bench in collect_benchmarks() {
        (bench.work)(1);
    }
}

const SAMPLES: usize = 100;
const MIN_DURATION: Duration = Duration::from_millis(3);
const MIN_REPEATS: u32 = 1_000;

struct Benchmark {
    label: String,
    work: Box<dyn FnMut(u32)>,
}

struct NanosPerOp {
    min: f64,
    p10: f64,
    p50: f64,
    p90: f64,
    max: f64,
    min_repeats: u32,
    max_repeats: u32,
}

fn run_benchmark(mut bench: Benchmark) -> NanosPerOp {
    let mut times = Vec::with_capacity(SAMPLES);
    let mut min_repeats = u32::MAX;
    let mut max_repeats = 0;
    for _ in 0..SAMPLES {
        let (dt, repeats) = one_sample(&mut bench);
        min_repeats = cmp::min(min_repeats, repeats);
        max_repeats = cmp::max(max_repeats, repeats);
        times.push((dt.as_nanos() as f64) / (repeats as f64));
    }
    times.sort_by(f64::total_cmp);
    let n = times.len();
    NanosPerOp {
        min: times[0],
        p10: times[n / 10],
        p50: times[n / 2],
        p90: times[(n * 9) / 10],
        max: times[n - 1],
        min_repeats,
        max_repeats,
    }
}

fn one_sample(bench: &mut Benchmark) -> (Duration, u32) {
    let mut repeats = MIN_REPEATS;
    loop {
        let t0 = Instant::now();
        (bench.work)(repeats);
        let dt = t0.elapsed();
        if dt >= MIN_DURATION {
            return (dt, repeats);
        }
        let Some(more) = repeats.checked_mul(2) else {
            eprintln!(
                "warning: benchmark {} did not reach min. duration after {} repeats",
                bench.label, repeats
            );
            return (dt, repeats);
        };
        repeats = more;
    }
}

const SEED: &[u8; 32] = b"thisisjustabenchthisisjustabench";

fn bench_next_u32(backend_name: &str, backend: Backend) -> Benchmark {
    let backend = black_box(backend);
    let mut rng = ChaCha8::with_backend(Seed::from(SEED), backend);
    Benchmark {
        label: format!("next_u32/{backend_name}"),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.next_u32());
            }
        }),
    }
}

fn bench_bulk(backend_name: &str, backend: Backend) -> Benchmark {
    let mut key = array::from_fn(|i| u32::from_le_bytes(*array_ref![SEED, i * 4, 4]));
    let mut buf = Buffer { words: [0; 256] };
    Benchmark {
        label: format!("bulk/{backend_name}"),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(&mut key);
                backend.refill(&key, &mut buf);
                key = *array_ref![buf.words, 256 - 8, 8];
                black_box(&buf);
            }
        }),
    }
}
