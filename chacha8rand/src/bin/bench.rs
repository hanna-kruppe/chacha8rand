use std::{
    cmp,
    hint::black_box,
    time::{Duration, Instant},
};

use chacha8rand::{Backend, ChaCha8Rand, Seed};

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
    let mut backends = Vec::new();
    backends.push(("scalar", Backend::scalar()));
    if let Some(sse2) = Backend::x86_sse2() {
        backends.push(("sse2", sse2));
    }
    if let Some(avx2) = Backend::x86_avx2() {
        backends.push(("avx2", avx2));
    }
    if let Some(neon) = Backend::aarch64_neon() {
        backends.push(("neon", neon));
    }
    if let Some(simd128) = Backend::wasm32_simd128() {
        backends.push(("simd128", simd128));
    }

    let mut benchmarks = Vec::new();

    for (backend_name, backend) in &backends {
        benchmarks.push(bench_next_u32(backend_name, *backend));
    }

    for (backend_name, backend) in &backends {
        // Each iteration generates 1024 bytes internally, but 32 of those are new key material, so
        // reading 1024 - 32 bytes is the "best case" buffer size. For comparison and to exercise
        // the partial read code path, we also benchmark with an odd buffer size that's as close to
        // 10% of the larger size as possible.
        for read_size in [99, 1024 - 32] {
            benchmarks.push(bench_bulk(backend_name, *backend, vec![0; read_size]))
        }
    }

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
    let mut rng = ChaCha8Rand::with_backend(Seed::from(SEED), backend);
    Benchmark {
        label: format!("next_u32/{backend_name}"),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.next_u32());
            }
        }),
    }
}

fn bench_bulk(backend_name: &str, backend: Backend, mut dest: Vec<u8>) -> Benchmark {
    let label = format!("bulk/{n}/{backend_name}", n = dest.len());
    Benchmark {
        label,
        work: Box::new(move |n| {
            let mut rng = ChaCha8Rand::with_backend(Seed::from(SEED), backend);
            for _ in 0..n {
                rng.read_bytes(&mut dest);
                black_box(&mut dest);
            }
        }),
    }
}
