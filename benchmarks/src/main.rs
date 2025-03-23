use std::{
    cmp,
    hint::black_box,
    time::{Duration, Instant},
};

use chacha8rand::ChaCha8Rand;
use rand_core::{RngCore, SeedableRng};

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
    let mut benchmarks = vec![
        bench_u32s(0),
        bench_u32s(1),
        bench_next_u32_rand_chacha(),
        bench_u64s(),
        bench_next_u64_rand_chacha(),
    ];

    // Each iteration generates 1024 bytes internally, but 32 of those are new key material, so
    // reading 1024 - 32 bytes is the "best case" buffer size for us. For a fairer comparison with
    // `chacha8_rand` and to exercise the partial read code path, we also benchmark with:
    // * 16 bytes (smallest interesting read size, probably)
    // * 99 bytes (as close to 10% of our ideal output size as possible)
    // * 1024 bytes (more favorable for `rand_chacha`, less favorable for us)
    let interesting_read_sizes = [16, 99, 1024 - 32, 1024];
    for read_size in interesting_read_sizes {
        benchmarks.push(bench_bulk(vec![0; read_size]));
        benchmarks.push(bench_bulk_rand_chacha(vec![0; read_size]));
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

fn bench_u32s(skip_first_bytes: usize) -> Benchmark {
    let mut rng = ChaCha8Rand::new(SEED);

    // Skipping a 1, 2, or 3 number of bytes means that `read_u32` calls that need to refill the
    // buffer *might* get slower because they do two small memcpys instead of one.
    let mut skip_buf = [0; 4];
    rng.read_bytes(&mut skip_buf[..skip_first_bytes]);

    Benchmark {
        label: format!("u32/skip{skip_first_bytes}b"),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.read_u32());
            }
        }),
    }
}

fn bench_next_u32_rand_chacha() -> Benchmark {
    let mut rng = rand_chacha::ChaCha8Rng::from_seed(*SEED);
    Benchmark {
        label: "u32/rand_chacha".to_string(),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.next_u32());
            }
        }),
    }
}

fn bench_u64s() -> Benchmark {
    let mut rng = ChaCha8Rand::new(SEED);

    Benchmark {
        label: format!("u64"),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.read_u64());
            }
        }),
    }
}

fn bench_next_u64_rand_chacha() -> Benchmark {
    let mut rng = rand_chacha::ChaCha8Rng::from_seed(*SEED);
    Benchmark {
        label: "u64/rand_chacha".to_string(),
        work: Box::new(move |n| {
            for _ in 0..n {
                black_box(rng.next_u64());
            }
        }),
    }
}

fn bench_bulk(mut dest: Vec<u8>) -> Benchmark {
    let label = format!("bulk{n}", n = dest.len());
    Benchmark {
        label,
        work: Box::new(move |n| {
            let mut rng = ChaCha8Rand::new(SEED);
            for _ in 0..n {
                rng.read_bytes(&mut dest);
                black_box(&mut dest);
            }
        }),
    }
}

fn bench_bulk_rand_chacha(mut dest: Vec<u8>) -> Benchmark {
    let label = format!("bulk{n}/rand_chacha", n = dest.len());
    Benchmark {
        label,
        work: Box::new(move |n| {
            let mut rng = rand_chacha::ChaCha8Rng::from_seed(*SEED);
            for _ in 0..n {
                rng.fill_bytes(&mut dest);
                black_box(&mut dest);
            }
        }),
    }
}
